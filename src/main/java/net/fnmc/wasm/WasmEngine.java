package net.fnmc.wasm;

import com.dylibso.chicory.runtime.ExportFunction;
import com.dylibso.chicory.runtime.HostFunction;
import com.dylibso.chicory.runtime.Instance;
import com.dylibso.chicory.runtime.Store;
import com.dylibso.chicory.wasm.Parser;
import com.dylibso.chicory.wasm.WasmModule;
import com.dylibso.chicory.wasm.types.FunctionType;
import com.dylibso.chicory.wasm.types.ValType;
import com.dylibso.chicory.wasm.types.Value;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.List;

/**
 * One instance of the Rust mechanics (rust/crates/fn-mc, wasm32) running in Chicory. Commands and answers are JSON (see the Rust {@code Cmd} / {@code Out}).
 * Not thread safe: use it from the server thread only.
 */
public final class WasmEngine {
	/** The host world query the building code asks: is any block with collision inside this box (metres)? */
	@FunctionalInterface
	public interface SolidQuery {
		boolean anySolid(double minX, double minY, double minZ, double maxX, double maxY, double maxZ);
	}

	private static WasmModule module;

	private static synchronized WasmModule module() {
		if (module == null) {
			try (InputStream in = WasmEngine.class.getResourceAsStream("/fnmc/fn_mc.wasm")) {
				if (in == null) throw new IllegalStateException("fnmc/fn_mc.wasm is missing from the mod jar");
				module = Parser.parse(in.readAllBytes());
			} catch (IOException e) {
				throw new IllegalStateException("cannot read fn_mc.wasm", e);
			}
		}
		return module;
	}

	private final Instance instance;
	private final ExportFunction alloc;
	private final ExportFunction free;
	private final ExportFunction call;
	private final int handle;

	public WasmEngine(SolidQuery solid) {
		HostFunction solidBox = new HostFunction("fnmc", "solid_box",
				FunctionType.of(List.of(ValType.F64, ValType.F64, ValType.F64, ValType.F64, ValType.F64, ValType.F64), List.of(ValType.I32)),
				(inst, args) -> new long[] {solid.anySolid(Value.longToDouble(args[0]), Value.longToDouble(args[1]), Value.longToDouble(args[2]),
						Value.longToDouble(args[3]), Value.longToDouble(args[4]), Value.longToDouble(args[5])) ? 1L : 0L});
		Store store = new Store().addFunction(solidBox);
		this.instance = store.instantiate("fnmc", module());
		this.alloc = instance.export("fnmc_alloc");
		this.free = instance.export("fnmc_free");
		this.call = instance.export("fnmc_call");
		this.handle = (int) instance.export("fnmc_new").apply()[0];
	}

	public JsonObject call(JsonObject command) {
		byte[] in = command.toString().getBytes(StandardCharsets.UTF_8);
		int ptr = (int) alloc.apply(in.length)[0];
		instance.memory().write(ptr, in);
		long packed = call.apply(handle, ptr, in.length)[0];
		int outPtr = (int) (packed >>> 32);
		int outLen = (int) (packed & 0xffffffffL);
		String text = new String(instance.memory().readBytes(outPtr, outLen), StandardCharsets.UTF_8);
		free.apply(outPtr, outLen);
		return JsonParser.parseString(text).getAsJsonObject();
	}
}
