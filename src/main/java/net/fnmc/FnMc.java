package net.fnmc;

import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.networking.v1.PayloadTypeRegistry;
import net.fnmc.net.CollidersPayload;
import net.fnmc.net.InputPayload;
import net.fnmc.server.ServerHooks;
import net.minecraft.resources.Identifier;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/** Common entrypoint: payload types (both sides) and the server side of the mechanics. */
public final class FnMc implements ModInitializer {
	public static final String MOD_ID = "fnmc";
	public static final Logger LOG = LoggerFactory.getLogger(MOD_ID);

	public static Identifier id(String path) {
		return Identifier.fromNamespaceAndPath(MOD_ID, path);
	}

	@Override
	public void onInitialize() {
		PayloadTypeRegistry.serverboundPlay().register(InputPayload.TYPE, InputPayload.CODEC);
		PayloadTypeRegistry.clientboundPlay().register(CollidersPayload.TYPE, CollidersPayload.CODEC);
		ServerHooks.register();
		LOG.info("FN Mechanics loaded");
	}
}
