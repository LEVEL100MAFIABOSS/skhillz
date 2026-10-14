package net.fnmc.test;

import net.fabricmc.fabric.api.gametest.v1.GameTest;
import net.fnmc.Colliders;
import net.fnmc.server.BuildManager;
import net.minecraft.core.BlockPos;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.Display;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.MoverType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/** The whole chain in a real server: input -> Rust engine (WebAssembly) -> block displays + colliders -> entity collision -> pickaxe damage. */
public class FnMcGameTest {
	@GameTest(maxTicks = 100)
	public void wallIsBuiltBlocksMobsAndBreaksAfterEightSwings(GameTestHelper helper) {
		ServerLevel level = helper.getLevel();
		for (int x = -2; x <= 12; x++) for (int z = -2; z <= 12; z++) helper.setBlock(x, 0, z, Blocks.STONE);
		BlockPos feetBlock = helper.absolutePos(new BlockPos(1, 1, 5));
		Vec3 feet = Vec3.atBottomCenterOf(feetBlock);
		Vec3 eye = feet.add(0, 1.62, 0);
		Vec3 look = new Vec3(0.97, -0.24, 0).normalize();

		BuildManager m = BuildManager.get(level);
		m.clear();
		m.setGridY(feetBlock.getY());
		Entity viewer = helper.spawn(EntityTypes.ARMOR_STAND, new BlockPos(1, 1, 5));
		m.playerInput("gametest", viewer, eye, look, feet, 0, 0, false, false, false, false);
		m.playerInput("gametest", viewer, eye, look, feet, 0, 0, true, false, false, false);
		helper.assertTrue(m.pieceCount() == 1, "the wall was not placed (pieces: " + m.pieceCount() + ")");

		var displays = level.getEntitiesOfClass(Display.BlockDisplay.class, new AABB(feet, feet).inflate(12));
		helper.assertTrue(!displays.isEmpty(), "no block display was spawned for the wall");

		AABB wall = Colliders.all(level).values().iterator().next()[0];
		helper.assertTrue(wall.minX > feet.x, "the wall is not in front (east) of the player: " + wall);
		helper.assertTrue(Math.abs((wall.maxY - wall.minY) - 3.84) < 1.0E-3, "the wall is not 3.84 m tall: " + wall);

		// a pig pushed east stops at the wall (EntityCollideMixin)
		Mob pig = helper.spawnWithNoFreeWill(EntityTypes.PIG, new BlockPos(1, 1, 5));
		for (int i = 0; i < 60; i++) pig.move(MoverType.SELF, new Vec3(0.25, 0, 0));
		helper.assertTrue(pig.getBoundingBox().maxX <= wall.minX + 1.0E-3, "the pig walked through the wall: pig " + pig.getBoundingBox() + " wall " + wall);
		pig.discard();

		// finish building, then hold attack with a pickaxe at the wall
		for (int i = 0; i < 200; i++) m.tick();
		Vec3 centre = wall.getCenter();
		Vec3 aim = new Vec3(centre.x - eye.x, 0, centre.z - eye.z).normalize();
		int ticks = 0;
		while (m.pieceCount() > 0 && ticks++ < 400) {
			m.playerInput("gametest", viewer, eye, aim, feet, -1, 0, false, false, true, true);
			m.tick();
		}
		helper.assertTrue(m.pieceCount() == 0, "the wall survived " + ticks + " ticks of pickaxe swings");
		helper.assertTrue(Colliders.all(level).isEmpty(), "the wall's collision boxes were not removed");
		helper.assertTrue(level.getEntitiesOfClass(Display.BlockDisplay.class, new AABB(feet, feet).inflate(12)).isEmpty(), "the wall's displays were not removed");
		helper.succeed();
	}

	@GameTest
	public void everyPieceAndMaterialPlaces(GameTestHelper helper) {
		ServerLevel level = helper.getLevel();
		for (int x = -2; x <= 12; x++) for (int z = -2; z <= 12; z++) helper.setBlock(x, 0, z, Blocks.STONE);
		BlockPos feetBlock = helper.absolutePos(new BlockPos(1, 1, 5));
		Vec3 feet = Vec3.atBottomCenterOf(feetBlock);
		Vec3 eye = feet.add(0, 1.62, 0);
		BuildManager m = BuildManager.get(level);
		m.clear();
		m.setGridY(feetBlock.getY());
		Entity viewer = helper.spawn(EntityTypes.ARMOR_STAND, new BlockPos(1, 1, 5));
		Vec3 look = new Vec3(0.9, -0.43, 0).normalize();
		int placed = 0;
		for (int piece = 0; piece < 4; piece++) {
			m.clear();
			m.playerInput("gametest", viewer, eye, look, feet, piece, piece % 3, false, false, false, false);
			m.playerInput("gametest", viewer, eye, look, feet, piece, piece % 3, true, false, false, false);
			if (m.pieceCount() == 1) placed++;
		}
		m.clear();
		helper.assertTrue(placed == 4, "only " + placed + " of 4 piece kinds could be placed on flat ground");
		helper.succeed();
	}
}
