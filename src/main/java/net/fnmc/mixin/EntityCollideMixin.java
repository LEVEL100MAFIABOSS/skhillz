package net.fnmc.mixin;

import com.llamalad7.mixinextras.injector.ModifyExpressionValue;
import com.llamalad7.mixinextras.sugar.Local;
import net.fnmc.Colliders;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.VoxelShape;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

import java.util.ArrayList;
import java.util.List;

/** Adds the Fortnite piece boxes to the shapes an entity's movement collides with (client prediction and server agree). */
@Mixin(Entity.class)
abstract class EntityCollideMixin {
	@ModifyExpressionValue(method = "collide", at = @At(value = "INVOKE",
			target = "Lnet/minecraft/world/level/Level;getEntityCollisions(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/world/phys/AABB;)Ljava/util/List;"))
	private List<VoxelShape> fnmc$addPieces(List<VoxelShape> original, @Local(argsOnly = true) Vec3 movement) {
		Entity self = (Entity) (Object) this;
		List<VoxelShape> pieces = Colliders.shapes(self.level(), self.getBoundingBox().expandTowards(movement).inflate(1.0E-7));
		if (pieces.isEmpty()) return original;
		List<VoxelShape> all = new ArrayList<>(original.size() + pieces.size());
		all.addAll(original);
		all.addAll(pieces);
		return all;
	}
}
