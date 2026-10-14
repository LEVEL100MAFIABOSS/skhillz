package net.fnmc.server;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.mojang.math.Transformation;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.fabricmc.loader.api.FabricLoader;
import net.fnmc.Colliders;
import net.fnmc.FnMc;
import net.fnmc.net.CollidersPayload;
import net.fnmc.wasm.WasmEngine;
import net.minecraft.core.BlockPos;
import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.Display;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.storage.LevelResource;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;
import org.joml.Quaternionf;
import org.joml.Vector3f;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;

/** The Fortnite building world of one Minecraft dimension: the Rust engine plus its block displays and collision boxes. */
public final class BuildManager {
	public static final String TAG = "fnmc";
	public static final String[] PIECES = {"wall", "floor", "stairs", "roof"};
	public static final String[] MATERIALS = {"wood", "stone", "metal"};
	public static final String[] PIECE_NAMES = {"Wall", "Floor", "Ramp", "Cone"};
	public static final String[] MATERIAL_NAMES = {"Wood", "Brick", "Metal"};
	/** How far the build preview looks for Minecraft blocks to build on (metres). */
	public static final double BUILD_REACH = 8.0;

	/** Every display entity this server session spawned; anything else tagged {@link #TAG} is stale and removed on load. */
	static final Set<UUID> LIVE = new HashSet<>();

	private static final Map<ServerLevel, BuildManager> MANAGERS = new HashMap<>();

	private static final class PieceView {
		final String material;
		final JsonArray boxes;
		final BlockPos anchor;
		final List<Entity> displays = new ArrayList<>();
		PieceView(String material, JsonArray boxes, BlockPos anchor) { this.material = material; this.boxes = boxes; this.anchor = anchor; }
	}

	final ServerLevel level;
	private final WasmEngine engine;
	private final Map<Integer, PieceView> pieces = new LinkedHashMap<>();
	private final Map<String, List<Entity>> previews = new HashMap<>();
	private final Map<UUID, Integer> displayOwner = new HashMap<>();
	private Double gridY;
	private final Path saveFile;

	public static BuildManager get(ServerLevel level) {
		return MANAGERS.computeIfAbsent(level, BuildManager::new);
	}

	public static Iterable<BuildManager> all() {
		return new ArrayList<>(MANAGERS.values());
	}

	static void closeAll() {
		for (BuildManager m : MANAGERS.values()) m.close();
		MANAGERS.clear();
		LIVE.clear();
	}

	private BuildManager(ServerLevel level) {
		this.level = level;
		this.engine = new WasmEngine((x0, y0, z0, x1, y1, z1) -> level.getBlockCollisions(null, new AABB(x0, y0, z0, x1, y1, z1)).iterator().hasNext());
		String dim = level.dimension().toString().toLowerCase().replaceAll("[^a-z0-9]+", "_");
		this.saveFile = level.getServer().getWorldPath(LevelResource.ROOT).resolve("fnmc").resolve(dim + ".json");
		JsonObject init = new JsonObject();
		init.addProperty("cmd", "init");
		Path data = FabricLoader.getInstance().getConfigDir().resolve("fnmc").resolve("mechanics-data.json");
		if (Files.isRegularFile(data)) {
			try {
				init.addProperty("data", Files.readString(data, StandardCharsets.UTF_8));
				FnMc.LOG.info("FN Mechanics: using {}", data);
			} catch (IOException e) {
				FnMc.LOG.warn("FN Mechanics: cannot read {}: {}", data, e.toString());
			}
		}
		apply(engine.call(init), null);
		load();
	}

	// ---- commands to the engine ------------------------------------------------------------------------------------------------

	public JsonObject command(JsonObject cmd) {
		JsonObject out = engine.call(cmd);
		apply(out, null);
		return out;
	}

	public void setGridY(double y) {
		gridY = y;
		JsonObject c = new JsonObject();
		c.addProperty("cmd", "set_grid_y");
		c.addProperty("y", y);
		command(c);
	}

	public Double gridY() { return gridY; }

	public int pieceCount() { return pieces.size(); }

	public void clear() {
		JsonObject c = new JsonObject();
		c.addProperty("cmd", "clear");
		command(c);
	}

	/** Advance build-up and structural collapse by one server tick. */
	public void tick() {
		JsonObject c = new JsonObject();
		c.addProperty("cmd", "tick");
		c.addProperty("dt", 0.05);
		command(c);
	}

	/** One player's input for this tick (eye, look and feet come from the player; buttons from {@code InputPayload}). {@code viewer}: the player (the block raycast's entity; messages go to it when it is a player). */
	public void playerInput(String id, Entity viewer, Vec3 eye, Vec3 look, Vec3 feet, int piece, int material, boolean place, boolean rotate, boolean attack, boolean pickaxe) {
		if (piece >= 0 && gridY == null) setGridY(Math.floor(feet.y)); // building level 0 = where the first builder stands
		JsonObject c = new JsonObject();
		c.addProperty("cmd", "player");
		c.addProperty("id", id);
		c.add("eye", vec(eye));
		c.add("look", vec(look));
		c.add("feet", vec(feet));
		if (piece >= 0) {
			c.addProperty("piece", PIECES[Math.min(piece, 3)]);
			BlockHitResult hit = level.clip(new ClipContext(eye, eye.add(look.scale(BUILD_REACH)), ClipContext.Block.COLLIDER, ClipContext.Fluid.NONE, viewer));
			if (hit.getType() == HitResult.Type.BLOCK) {
				c.add("hit", vec(hit.getLocation()));
				c.add("hit_normal", vec(new Vec3(hit.getDirection().getStepX(), hit.getDirection().getStepY(), hit.getDirection().getStepZ())));
			}
		}
		c.addProperty("material", MATERIALS[Math.floorMod(material, 3)]);
		c.addProperty("place", place);
		c.addProperty("rotate", rotate);
		c.addProperty("attack", attack);
		c.addProperty("pickaxe", pickaxe);
		c.addProperty("dt", 0.05);
		apply(engine.call(c), viewer instanceof ServerPlayer sp ? sp : null);
	}

	public void removePlayer(String id) {
		JsonObject c = new JsonObject();
		c.addProperty("cmd", "remove_player");
		c.addProperty("id", id);
		command(c);
	}

	private static JsonArray vec(Vec3 v) {
		JsonArray a = new JsonArray();
		a.add(v.x); a.add(v.y); a.add(v.z);
		return a;
	}

	// ---- engine output -> Minecraft --------------------------------------------------------------------------------------------

	private void apply(JsonObject out, ServerPlayer actor) {
		if (out.has("error")) FnMc.LOG.warn("FN Mechanics engine: {}", out.get("error").getAsString());
		Map<Integer, AABB[]> added = new LinkedHashMap<>();
		List<Integer> removed = new ArrayList<>();
		for (JsonElement e : out.getAsJsonArray("remove")) {
			int id = e.getAsInt();
			PieceView v = pieces.remove(id);
			if (v != null) discard(v.displays);
			Colliders.remove(level, id);
			removed.add(id);
		}
		for (JsonElement e : out.getAsJsonArray("spawn")) {
			JsonObject s = e.getAsJsonObject();
			int id = s.get("id").getAsInt();
			JsonArray boxes = s.getAsJsonArray("boxes");
			JsonArray pos = boxes.get(0).getAsJsonObject().getAsJsonArray("pos");
			PieceView v = new PieceView(s.get("material").getAsString(), boxes, BlockPos.containing(pos.get(0).getAsDouble(), pos.get(1).getAsDouble(), pos.get(2).getAsDouble()));
			pieces.put(id, v);
			AABB[] aabbs = aabbs(s.getAsJsonArray("aabbs"));
			Colliders.put(level, id, aabbs);
			added.put(id, aabbs);
			removed.remove((Integer) id);
			spawnDisplays(id, v);
		}
		if (!added.isEmpty() || !removed.isEmpty()) {
			CollidersPayload p = new CollidersPayload(false, added, removed);
			for (ServerPlayer pl : level.players()) if (ServerPlayNetworking.canSend(pl, CollidersPayload.TYPE)) ServerPlayNetworking.send(pl, p);
		}
		for (JsonElement e : out.getAsJsonArray("hits")) {
			JsonObject h = e.getAsJsonObject();
			JsonArray p = h.getAsJsonArray("pos");
			String material = h.get("material").getAsString();
			boolean destroyed = h.get("destroyed").getAsBoolean();
			level.playSound(null, p.get(0).getAsDouble(), p.get(1).getAsDouble(), p.get(2).getAsDouble(), sound(material, destroyed), SoundSource.BLOCKS, 1.0f, 1.0f);
			if (actor != null) {
				actor.sendOverlayMessage(Component.literal(destroyed ? "Destroyed!" : String.format("%s  %d / %d", MATERIAL_NAMES[materialIndex(material)], Math.round(h.get("health").getAsFloat()), Math.round(h.get("max_health").getAsFloat()))));
			}
		}
		for (JsonElement e : out.getAsJsonArray("preview_clear")) discard(previews.remove(e.getAsString()));
		for (JsonElement e : out.getAsJsonArray("preview")) {
			JsonObject pv = e.getAsJsonObject();
			String player = pv.get("player").getAsString();
			discard(previews.remove(player));
			BlockState ghost = (pv.get("ok").getAsBoolean() ? Blocks.LIGHT_BLUE_STAINED_GLASS : Blocks.RED_STAINED_GLASS).defaultBlockState();
			List<Entity> list = new ArrayList<>();
			for (JsonElement b : pv.getAsJsonArray("boxes")) {
				Entity d = spawnBox(b.getAsJsonObject(), ghost, true);
				if (d != null) list.add(d);
			}
			previews.put(player, list);
		}
		for (JsonElement e : out.getAsJsonArray("messages")) {
			JsonArray m = e.getAsJsonArray();
			ServerPlayer p = player(m.get(0).getAsString());
			if (p != null) p.sendOverlayMessage(Component.literal(m.get(1).getAsString()));
		}
	}

	private ServerPlayer player(String uuid) {
		try {
			Entity e = level.getEntity(UUID.fromString(uuid));
			return e instanceof ServerPlayer sp ? sp : null;
		} catch (IllegalArgumentException ex) {
			return null;
		}
	}

	static int materialIndex(String m) {
		return switch (m) { case "stone" -> 1; case "metal" -> 2; default -> 0; };
	}

	private static SoundEvent sound(String material, boolean destroyed) {
		return switch (material) {
			case "stone" -> destroyed ? SoundEvents.STONE_BREAK : SoundEvents.STONE_HIT;
			case "metal" -> destroyed ? SoundEvents.METAL_BREAK : SoundEvents.METAL_HIT;
			default -> destroyed ? SoundEvents.WOOD_BREAK : SoundEvents.WOOD_HIT;
		};
	}

	private static BlockState block(String material) {
		return switch (material) {
			case "stone" -> Blocks.STONE_BRICKS.defaultBlockState();
			case "metal" -> Blocks.IRON_BLOCK.defaultBlockState();
			default -> Blocks.OAK_PLANKS.defaultBlockState();
		};
	}

	private static AABB[] aabbs(JsonArray a) {
		AABB[] out = new AABB[a.size()];
		for (int i = 0; i < out.length; i++) {
			JsonArray b = a.get(i).getAsJsonArray();
			out[i] = new AABB(b.get(0).getAsDouble(), b.get(1).getAsDouble(), b.get(2).getAsDouble(), b.get(3).getAsDouble(), b.get(4).getAsDouble(), b.get(5).getAsDouble());
		}
		return out;
	}

	private void spawnDisplays(int id, PieceView v) {
		if (!v.displays.isEmpty() || !level.isLoaded(v.anchor)) return;
		BlockState state = block(v.material);
		for (JsonElement b : v.boxes) {
			Entity d = spawnBox(b.getAsJsonObject(), state, false);
			if (d != null) { v.displays.add(d); displayOwner.put(d.getUUID(), id); }
		}
	}

	private Entity spawnBox(JsonObject b, BlockState state, boolean glow) {
		Display.BlockDisplay d = EntityTypes.BLOCK_DISPLAY.create(level, EntitySpawnReason.TRIGGERED);
		if (d == null) return null;
		JsonArray p = b.getAsJsonArray("pos"), t = b.getAsJsonArray("translation"), r = b.getAsJsonArray("rot"), s = b.getAsJsonArray("scale");
		d.setPos(p.get(0).getAsDouble(), p.get(1).getAsDouble(), p.get(2).getAsDouble());
		d.setBlockState(state);
		d.setTransformation(new Transformation(
				new Vector3f(t.get(0).getAsFloat(), t.get(1).getAsFloat(), t.get(2).getAsFloat()),
				new Quaternionf(r.get(0).getAsFloat(), r.get(1).getAsFloat(), r.get(2).getAsFloat(), r.get(3).getAsFloat()),
				new Vector3f(s.get(0).getAsFloat(), s.get(1).getAsFloat(), s.get(2).getAsFloat()),
				new Quaternionf()));
		d.setViewRange(2.0f);
		if (glow) d.setGlowingTag(true);
		d.addTag(TAG);
		LIVE.add(d.getUUID());
		level.addFreshEntity(d);
		return d;
	}

	private void discard(List<Entity> list) {
		if (list == null) return;
		for (Entity e : list) {
			LIVE.remove(e.getUUID());
			displayOwner.remove(e.getUUID());
			e.discard();
		}
		list.clear();
	}

	/** A display of ours left with its chunk: it is saved to disk (and removed when loaded again); spawn fresh ones when the chunk is back. */
	void onDisplayUnloaded(Entity e) {
		LIVE.remove(e.getUUID());
		Integer id = displayOwner.remove(e.getUUID());
		if (id != null && pieces.containsKey(id)) pieces.get(id).displays.remove(e);
		previews.values().forEach(l -> l.remove(e));
	}

	/** Once a second: pieces whose chunk is loaded but have no displays get them. */
	void respawnMissing() {
		for (Map.Entry<Integer, PieceView> e : pieces.entrySet()) {
			if (e.getValue().displays.isEmpty()) spawnDisplays(e.getKey(), e.getValue());
		}
	}

	/** Everything a player in this level needs to collide with the pieces (on join / level change). */
	void sendAllColliders(ServerPlayer p) {
		if (ServerPlayNetworking.canSend(p, CollidersPayload.TYPE)) ServerPlayNetworking.send(p, new CollidersPayload(true, Colliders.all(level), List.of()));
	}

	// ---- saving ----------------------------------------------------------------------------------------------------------------

	void save() {
		JsonObject c = new JsonObject();
		c.addProperty("cmd", "export");
		JsonObject out = engine.call(c);
		JsonObject file = new JsonObject();
		if (gridY != null) file.addProperty("grid_y", gridY);
		file.add("pieces", out.getAsJsonArray("export"));
		try {
			Files.createDirectories(saveFile.getParent());
			Files.writeString(saveFile, file.toString(), StandardCharsets.UTF_8);
		} catch (IOException e) {
			FnMc.LOG.warn("FN Mechanics: cannot save {}: {}", saveFile, e.toString());
		}
	}

	private void load() {
		if (!Files.isRegularFile(saveFile)) return;
		try {
			JsonObject file = JsonParser.parseString(Files.readString(saveFile, StandardCharsets.UTF_8)).getAsJsonObject();
			if (file.has("grid_y")) setGridY(file.get("grid_y").getAsDouble());
			JsonObject c = new JsonObject();
			c.addProperty("cmd", "import");
			c.add("pieces", file.getAsJsonArray("pieces"));
			command(c);
			FnMc.LOG.info("FN Mechanics: restored {} pieces in {}", pieces.size(), level.dimension());
		} catch (Exception e) {
			FnMc.LOG.warn("FN Mechanics: cannot load {}: {}", saveFile, e.toString());
		}
	}

	private void close() {
		save();
		for (PieceView v : pieces.values()) discard(v.displays);
		for (List<Entity> l : previews.values()) discard(l);
		previews.clear();
		Colliders.clear(level);
	}
}
