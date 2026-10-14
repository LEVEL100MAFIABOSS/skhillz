package net.fnmc;

import net.minecraft.world.level.Level;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.WeakHashMap;

/**
 * Collision boxes of the Fortnite pieces, per level (server levels are fed by the Rust engine, client levels by {@code CollidersPayload}),
 * with a 16 x 16 column index. {@code EntityCollideMixin} adds them to every entity's movement collision, on both sides, so players are
 * blocked by walls and walk up ramps with client-side prediction matching the server.
 */
public final class Colliders {
	private static final class Set_ {
		final Map<Integer, AABB[]> byPiece = new HashMap<>();
		final Map<Long, Set<Integer>> byColumn = new HashMap<>();
	}

	private static final Map<Level, Set_> LEVELS = Collections.synchronizedMap(new WeakHashMap<>());

	private Colliders() {}

	private static long column(int cx, int cz) {
		return ((long) cx << 32) ^ (cz & 0xffffffffL);
	}

	private static Set_ of(Level level) {
		return LEVELS.computeIfAbsent(level, l -> new Set_());
	}

	public static synchronized void put(Level level, int id, AABB[] boxes) {
		remove(level, id);
		Set_ s = of(level);
		s.byPiece.put(id, boxes);
		for (AABB b : boxes) {
			for (int cx = (int) Math.floor(b.minX) >> 4; cx <= (int) Math.floor(b.maxX) >> 4; cx++) {
				for (int cz = (int) Math.floor(b.minZ) >> 4; cz <= (int) Math.floor(b.maxZ) >> 4; cz++) {
					s.byColumn.computeIfAbsent(column(cx, cz), k -> new HashSet<>()).add(id);
				}
			}
		}
	}

	public static synchronized void remove(Level level, int id) {
		Set_ s = LEVELS.get(level);
		if (s == null || s.byPiece.remove(id) == null) return;
		s.byColumn.values().forEach(ids -> ids.remove(id));
		s.byColumn.values().removeIf(Set::isEmpty);
	}

	public static synchronized void clear(Level level) {
		LEVELS.remove(level);
	}

	public static synchronized Map<Integer, AABB[]> all(Level level) {
		Set_ s = LEVELS.get(level);
		return s == null ? Map.of() : new HashMap<>(s.byPiece);
	}

	private static Set<Integer> near(Set_ s, AABB area) {
		Set<Integer> ids = new HashSet<>();
		for (int cx = (int) Math.floor(area.minX) >> 4; cx <= (int) Math.floor(area.maxX) >> 4; cx++) {
			for (int cz = (int) Math.floor(area.minZ) >> 4; cz <= (int) Math.floor(area.maxZ) >> 4; cz++) {
				Set<Integer> c = s.byColumn.get(column(cx, cz));
				if (c != null) ids.addAll(c);
			}
		}
		return ids;
	}

	/** The piece boxes touching {@code area}, as collision shapes. */
	public static synchronized List<VoxelShape> shapes(Level level, AABB area) {
		Set_ s = LEVELS.get(level);
		if (s == null || s.byPiece.isEmpty()) return List.of();
		List<VoxelShape> out = new ArrayList<>();
		for (int id : near(s, area)) {
			for (AABB b : s.byPiece.get(id)) {
				if (b.intersects(area)) out.add(Shapes.create(b));
			}
		}
		return out;
	}

	/** Distance along {@code from -> to} to the first piece box, or {@code Double.POSITIVE_INFINITY}. */
	public static synchronized double rayDistance(Level level, Vec3 from, Vec3 to) {
		Set_ s = LEVELS.get(level);
		if (s == null || s.byPiece.isEmpty()) return Double.POSITIVE_INFINITY;
		double best = Double.POSITIVE_INFINITY;
		for (int id : near(s, new AABB(from, to))) {
			for (AABB b : s.byPiece.get(id)) {
				Optional<Vec3> hit = b.clip(from, to);
				if (hit.isPresent()) best = Math.min(best, hit.get().distanceTo(from));
			}
		}
		return best;
	}
}
