package net.fnmc.net;

import net.fnmc.FnMc;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;

/**
 * Client -> server every tick: the build selection and buttons. {@code piece}: -1 not building, 0 wall, 1 floor, 2 ramp, 3 cone;
 * {@code material}: 0 wood, 1 brick, 2 metal; {@code place} / {@code rotate}: presses this tick; {@code attack}: attack button held.
 */
public record InputPayload(int piece, int material, boolean place, boolean rotate, boolean attack) implements CustomPacketPayload {
	public static final CustomPacketPayload.Type<InputPayload> TYPE = new CustomPacketPayload.Type<>(FnMc.id("input"));

	public static final StreamCodec<RegistryFriendlyByteBuf, InputPayload> CODEC = StreamCodec.composite(
			ByteBufCodecs.VAR_INT, InputPayload::piece,
			ByteBufCodecs.VAR_INT, InputPayload::material,
			ByteBufCodecs.BOOL, InputPayload::place,
			ByteBufCodecs.BOOL, InputPayload::rotate,
			ByteBufCodecs.BOOL, InputPayload::attack,
			InputPayload::new);

	@Override
	public Type<? extends CustomPacketPayload> type() {
		return TYPE;
	}
}
