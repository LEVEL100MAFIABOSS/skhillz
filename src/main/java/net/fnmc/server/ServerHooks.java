package net.fnmc.server;

import com.mojang.brigadier.arguments.IntegerArgumentType;
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerEntityEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.event.player.AttackBlockCallback;
import net.fabricmc.fabric.api.networking.v1.ServerPlayConnectionEvents;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.fnmc.net.InputPayload;
import net.minecraft.commands.Commands;
import net.minecraft.network.chat.Component;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.tags.ItemTags;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.entity.Entity;

import java.util.HashMap;
import java.util.Map;
import java.util.UUID;

/** Server wiring: player input, the per-tick engine steps, cleanup of stale displays, saving and the /fnmc command. */
public final class ServerHooks {
	private static final class Input {
		int piece = -1, material;
		boolean place, rotate, attack;
		long lastSeen;
	}

	private static final Map<UUID, Input> INPUTS = new HashMap<>();
	private static final Map<UUID, ServerLevel> LAST_LEVEL = new HashMap<>();
	private static long ticks;

	private ServerHooks() {}

	public static void register() {
		ServerPlayNetworking.registerGlobalReceiver(InputPayload.TYPE, (payload, context) -> {
			Input in = INPUTS.computeIfAbsent(context.player().getUUID(), k -> new Input());
			in.piece = payload.piece();
			in.material = payload.material();
			in.place |= payload.place();
			in.rotate |= payload.rotate();
			in.attack = payload.attack();
			in.lastSeen = ticks;
		});

		ServerTickEvents.END_SERVER_TICK.register(ServerHooks::tick);

		ServerEntityEvents.ENTITY_LOAD.register((entity, level) -> {
			if (entity.entityTags().contains(BuildManager.TAG) && !BuildManager.LIVE.contains(entity.getUUID())) entity.discard();
		});
		ServerEntityEvents.ENTITY_UNLOAD.register((entity, level) -> {
			if (BuildManager.LIVE.contains(entity.getUUID())) BuildManager.get(level).onDisplayUnloaded(entity);
		});

		ServerPlayConnectionEvents.DISCONNECT.register((listener, server) -> {
			ServerPlayer p = listener.player;
			INPUTS.remove(p.getUUID());
			LAST_LEVEL.remove(p.getUUID());
			for (BuildManager m : BuildManager.all()) m.removePlayer(p.getStringUUID());
		});

		ServerLifecycleEvents.SERVER_STOPPING.register(server -> BuildManager.closeAll());
		ServerLifecycleEvents.BEFORE_SAVE.register((server, flush, force) -> { for (BuildManager m : BuildManager.all()) m.save(); });

		// Holding a pickaxe at a Fortnite piece: the pickaxe hits the piece, not the block behind it
		AttackBlockCallback.EVENT.register((player, level, hand, pos, direction) -> {
			if (blockedByPiece(player, level)) return InteractionResult.FAIL;
			return InteractionResult.PASS;
		});

		CommandRegistrationCallback.EVENT.register((dispatcher, buildContext, selection) -> dispatcher.register(Commands.literal("fnmc")
				.then(Commands.literal("info").executes(ctx -> {
					BuildManager m = BuildManager.get(ctx.getSource().getLevel());
					ctx.getSource().sendSuccess(() -> Component.literal("FN Mechanics: " + m.pieceCount() + " pieces here, building level 0 at y = " + (m.gridY() == null ? "(set by the first build)" : m.gridY())), false);
					return 1;
				}))
				.then(Commands.literal("clear").executes(ctx -> {
					BuildManager m = BuildManager.get(ctx.getSource().getLevel());
					int n = m.pieceCount();
					m.clear();
					ctx.getSource().sendSuccess(() -> Component.literal("Removed " + n + " pieces"), true);
					return n;
				}))
				.then(Commands.literal("grid")
						.then(Commands.literal("here").executes(ctx -> {
							double y = Math.floor(ctx.getSource().getPosition().y);
							BuildManager.get(ctx.getSource().getLevel()).setGridY(y);
							ctx.getSource().sendSuccess(() -> Component.literal("Building level 0 is now y = " + y), true);
							return 1;
						}))
						.then(Commands.argument("y", IntegerArgumentType.integer()).executes(ctx -> {
							int y = IntegerArgumentType.getInteger(ctx, "y");
							BuildManager.get(ctx.getSource().getLevel()).setGridY(y);
							ctx.getSource().sendSuccess(() -> Component.literal("Building level 0 is now y = " + y), true);
							return 1;
						})))));
	}

	static boolean blockedByPiece(net.minecraft.world.entity.player.Player player, net.minecraft.world.level.Level level) {
		if (!player.getMainHandItem().is(ItemTags.PICKAXES)) return false;
		var eye = player.getEyePosition();
		var to = eye.add(player.getLookAngle().scale(4.5));
		double piece = net.fnmc.Colliders.rayDistance(level, eye, to);
		if (piece == Double.POSITIVE_INFINITY) return false;
		var hit = level.clip(new net.minecraft.world.level.ClipContext(eye, to, net.minecraft.world.level.ClipContext.Block.OUTLINE, net.minecraft.world.level.ClipContext.Fluid.NONE, player));
		return piece < hit.getLocation().distanceTo(eye);
	}

	private static void tick(MinecraftServer server) {
		ticks++;
		for (ServerLevel level : server.getAllLevels()) {
			boolean any = false;
			for (ServerPlayer p : level.players()) {
				if (LAST_LEVEL.get(p.getUUID()) != level) {
					LAST_LEVEL.put(p.getUUID(), level);
					BuildManager.get(level).sendAllColliders(p);
				}
				Input in = INPUTS.get(p.getUUID());
				if (in == null || ticks - in.lastSeen > 40) continue;
				any = true;
				BuildManager m = BuildManager.get(level);
				Entity cam = p;
				m.playerInput(p.getStringUUID(), p, cam.getEyePosition(), cam.getLookAngle(), p.position(), in.piece, in.material, in.place, in.rotate, in.attack, p.getMainHandItem().is(ItemTags.PICKAXES));
				in.place = false;
				in.rotate = false;
				if (in.piece >= 0) {
					p.sendOverlayMessage(Component.literal("Building: " + BuildManager.PIECE_NAMES[Math.min(in.piece, 3)] + " (" + BuildManager.MATERIAL_NAMES[Math.floorMod(in.material, 3)] + ")   [click: place, right-click: material, R: rotate, B: exit]"));
				}
			}
			if (any || BuildManager.all().iterator().hasNext()) {
				for (BuildManager m : BuildManager.all()) if (m.level == level) m.tick();
			}
		}
		if (ticks % 20 == 0) for (BuildManager m : BuildManager.all()) m.respawnMissing();
	}
}
