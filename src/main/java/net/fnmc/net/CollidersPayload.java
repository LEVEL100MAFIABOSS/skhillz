package net.fnmc.net;

import net.fnmc.FnMc;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;
import net.minecraft.world.phys.AABB;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Server -> client: piece collision boxes for the player's current level ({@code reset}: forget all first). */
public record CollidersPayload(boolean reset, Map<Integer, AABB[]> add, List<Integer> remove) implements CustomPacketPayload {
	public static final CustomPacketPayload.Type<CollidersPayload> TYPE = new CustomPacketPayload.Type<>(FnMc.id("colliders"));

	public static final StreamCodec<RegistryFriendlyByteBuf, CollidersPayload> CODEC = StreamCodec.of(CollidersPayload::write, CollidersPayload::read);

	private static void write(RegistryFriendlyByteBuf buf, CollidersPayload p) {
		buf.writeBoolean(p.reset);
		buf.writeVarInt(p.add.size());
		for (Map.Entry<Integer, AABB[]> e : p.add.entrySet()) {
			buf.writeVarInt(e.getKey());
			buf.writeVarInt(e.getValue().length);
			for (AABB b : e.getValue()) {
				buf.writeDouble(b.minX); buf.writeDouble(b.minY); buf.writeDouble(b.minZ);
				buf.writeDouble(b.maxX); buf.writeDouble(b.maxY); buf.writeDouble(b.maxZ);
			}
		}
		buf.writeVarInt(p.remove.size());
		for (int id : p.remove) buf.writeVarInt(id);
	}

	private static CollidersPayload read(RegistryFriendlyByteBuf buf) {
		boolean reset = buf.readBoolean();
		int n = buf.readVarInt();
		Map<Integer, AABB[]> add = new LinkedHashMap<>();
		for (int i = 0; i < n; i++) {
			int id = buf.readVarInt();
			AABB[] boxes = new AABB[buf.readVarInt()];
			for (int k = 0; k < boxes.length; k++) {
				boxes[k] = new AABB(buf.readDouble(), buf.readDouble(), buf.readDouble(), buf.readDouble(), buf.readDouble(), buf.readDouble());
			}
			add.put(id, boxes);
		}
		int r = buf.readVarInt();
		List<Integer> remove = new ArrayList<>(r);
		for (int i = 0; i < r; i++) remove.add(buf.readVarInt());
		return new CollidersPayload(reset, add, remove);
	}

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
