package net.fnmc.client;

import com.mojang.blaze3d.platform.InputConstants;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
import net.fabricmc.fabric.api.event.player.AttackBlockCallback;
import net.fabricmc.fabric.api.event.player.AttackEntityCallback;
import net.fabricmc.fabric.api.event.player.UseBlockCallback;
import net.fabricmc.fabric.api.event.player.UseItemCallback;
import net.fnmc.Colliders;
import net.fnmc.FnMc;
import net.fnmc.net.CollidersPayload;
import net.fnmc.net.InputPayload;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.world.InteractionResult;
import org.lwjgl.sdl.SDLScancode;

import java.util.Map;

/**
 * Fortnite-style building controls. Z wall, X floor, C ramp, V cone (default keys, rebindable); while building, left click places,
 * right click changes material, R rotates, B leaves build mode. Holding a pickaxe and attacking hits pieces in front of you.
 */
public final class FnMcClient implements ClientModInitializer {
	private static final KeyMapping.Category CATEGORY = KeyMapping.Category.register(FnMc.id("main"));
	private static KeyMapping wall, floor, stairs, roof, rotate, exit;

	/** -1 = not building; 0 wall, 1 floor, 2 ramp, 3 cone. */
	static int piece = -1;
	static int material = 0;

	private static KeyMapping key(String name, int scancode) {
		return KeyMappingHelper.registerKeyMapping(new KeyMapping("key.fnmc." + name, InputConstants.Type.KEYBOARD, scancode, CATEGORY));
	}

	@Override
	public void onInitializeClient() {
		wall = key("wall", SDLScancode.SDL_SCANCODE_Z);
		floor = key("floor", SDLScancode.SDL_SCANCODE_X);
		stairs = key("stairs", SDLScancode.SDL_SCANCODE_C);
		roof = key("roof", SDLScancode.SDL_SCANCODE_V);
		rotate = key("rotate", SDLScancode.SDL_SCANCODE_R);
		exit = key("exit", SDLScancode.SDL_SCANCODE_B);

		ClientTickEvents.START_CLIENT_TICK.register(FnMcClient::tick);

		ClientPlayNetworking.registerGlobalReceiver(CollidersPayload.TYPE, (payload, context) -> {
			var level = context.client().level;
			if (level == null) return;
			if (payload.reset()) Colliders.clear(level);
			for (int id : payload.remove()) Colliders.remove(level, id);
			for (Map.Entry<Integer, net.minecraft.world.phys.AABB[]> e : payload.add().entrySet()) Colliders.put(level, e.getKey(), e.getValue());
		});

		// While building, clicks belong to building: no block breaking, attacking or using
		AttackBlockCallback.EVENT.register((player, level, hand, pos, direction) -> level.isClientSide() && piece >= 0 ? InteractionResult.FAIL : InteractionResult.PASS);
		AttackEntityCallback.EVENT.register((player, level, hand, entity, hit) -> level.isClientSide() && piece >= 0 ? InteractionResult.FAIL : InteractionResult.PASS);
		UseBlockCallback.EVENT.register((player, level, hand, hit) -> level.isClientSide() && piece >= 0 ? InteractionResult.FAIL : InteractionResult.PASS);
		UseItemCallback.EVENT.register((player, level, hand) -> level.isClientSide() && piece >= 0 ? InteractionResult.FAIL : InteractionResult.PASS);
	}

	private static void tick(Minecraft client) {
		if (client.player == null || client.level == null) { piece = -1; return; }
		while (wall.consumeClick()) piece = 0;
		while (floor.consumeClick()) piece = 1;
		while (stairs.consumeClick()) piece = 2;
		while (roof.consumeClick()) piece = 3;
		while (exit.consumeClick()) piece = -1;
		boolean place = false, turn = false;
		if (piece >= 0) {
			// take the clicks before vanilla handles them this tick
			while (client.options.keyAttack.consumeClick()) place = true;
			while (client.options.keyUse.consumeClick()) material = (material + 1) % 3;
			while (rotate.consumeClick()) turn = true;
		} else {
			while (rotate.consumeClick()) { }
		}
		boolean attack = client.options.keyAttack.isDown(); // menus release every key, so this is false while one is open
		if (ClientPlayNetworking.canSend(InputPayload.TYPE)) {
			ClientPlayNetworking.send(new InputPayload(piece, material, place, turn, attack));
		}
	}
}
