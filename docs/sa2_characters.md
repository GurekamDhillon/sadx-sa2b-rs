# Sonic Adventure 2 (PC Steam): Playable Character Controllers

Sources (docs):
- SA2Structs.h  https://github.com/X-Hax/sa2-mod-loader/blob/master/SA2ModLoader/include/SA2Structs.h
- SA2Variables.h https://github.com/X-Hax/sa2-mod-loader/blob/master/SA2ModLoader/include/SA2Variables.h
- SA2Functions.h https://github.com/X-Hax/sa2-mod-loader/blob/master/SA2ModLoader/include/SA2Functions.h
- SA2Enums.h https://github.com/X-Hax/sa2-mod-loader/blob/master/SA2ModLoader/include/SA2Enums.h
- SA Mod Manager (installs and configures the SA2 Mod Loader; it's not an address source): https://github.com/X-Hax/SA-Mod-Manager

Legend: [DOC] means the item comes from the mod loader headers and was not checked against the binary. [LOCAL] means it was verified on disk in this install.
I did not disassemble the binary in this pass, so every address below is [DOC]. The addresses target the Steam sonic2app.exe that the mod loader supports.

## 1. Object/task structure [DOC]
All structs use `#pragma pack(1)`.
- **ObjectMaster** (task): PrevObject, NextObject, Parent, Child, MainSub, DisplaySub, DeleteSub, DisplaySub_Delayed1..4, field_2C, SETData, Data1 (EntityData1*), EntityData2, UnknownA_ptr, Data2 (CharObj2Base* for players), Name, NameAgain, field_4C.
- **EntityData1** (taskwk): char Action; char NextAction; char field_2; char Index; s16 Status; s16 Timer; Rotation; NJS_VECTOR Position; NJS_VECTOR Scale; CollisionInfo* Collision.
- **CharObj2Base** (playerwk, ~0x1BC bytes): PlayerNum, CharID, Costume, CharID2, ActionWindowItems[8], ActionWindowItemCount, Powerups (s16), DisableControlTimer, UnderwaterTime, IdleTime, ConfuseTime, **int Upgrades** (+0x24), TiltAngle, PathDist, Up, SomeMultiplier, StoredSpeed, MechHP, Eff, Acceleration, Speed, WallNormal, FloorNormal, CurrentSurfaceFlags, PreviousSurfaceFlags, DynColInfo (csts*), HeldObject, HoldTarget, CurrentDyncolTask, MovementBuffer, PathData (LoopHead*), Animation (NJS_MOTION**), **PhysicsData PhysData** (per-object copy), SomeVectors[4], CharAnimInfo AnimInfo, CharSurfaceInfo SurfaceInfo.
- **SonicCharObj2** (Sonic/Shadow/Amy/Metal Sonic): base + bone positions/vectors (hands, feet, head), SomersaultNextAction, SomersaultTime, SpindashCounter, FlameRingPulse, BounceBraceletPulse, SpineJigglePos, LightDashPos, SpineJiggle, TextureList, ModelList (ModelIndex*), MotionList (AnimationIndex*).
- **KnucklesCharObj2** (Knuckles/Rouge): base + bone vecs, WingAction, WingAnimID, DigPos, WingFloat, Head/Torso JiggleInfo, TextureList, EffectTextureList, ModelList, MotionList, WingMotion, WingShapeMotion.
- **MechEggmanCharObj2** (Tails/Eggman mechs, Chao walkers): weapon/barrel/shot/lock-on laser vecs, rockettimer, BlasterAimAngle, MechTorsoTopAimAngle, LaserBlasterPulse, CannonUpgradeRecoil, BlasterBarrelRecoil, CommonTextureList, TextureList, ModelList, MotionList.
- The header also defines EggmanCharObj2 and TailsCharObj2 for the on-foot characters.

Globals [DOC]: `MainCharacter[8]` ObjectMaster* @0x1DEA6E0; `MainCharObj1[8]` EntityData1* @0x1DEA6C0; `MainCharObj2[8]` CharObj2Base* @0x1DE9600; `CurrentCharacter` int @0x1934B80.

## 2. Load / Main / Display functions [DOC]
| Function | Addr |
|---|---|
| LoadCharacters | 0x43D630 |
| LoadSonic | 0x716E00 |
| LoadAmy | 0x7170E0 |
| LoadShadow | 0x717360 |
| LoadMetalSonic | 0x717640 |
| Sonic_Main (shared Sonic/Shadow/Amy/Metal) | 0x717840 |
| Sonic_Display | 0x720090 |
| Sonic_Delete | 0x71E300 |
| Sonic_ChecksForDamage | 0x719920 |
| Sonic_CheckBounceAttack | 0x725270 |
| LoadKnuckles | 0x728110 |
| LoadRouge | 0x728460 |
| Knuckles_Main (Knux/Rouge) | 0x728D70 |
| Eggman_Main (on foot) | 0x73C380 |
| LoadMechEggman | 0x740C50 |
| LoadMechTails | 0x740EB0 |
| LoadChaoWalker | 0x741110 |
| LoadDarkChaoWalker | 0x7412F0 |
| MechEggman_Main (walkers) | 0x741530 |
| Tails_Main (on foot) | 0x74D170 |
| PlayerMovement | 0x472F70 |
| PGetAcceleration | 0x45B610 |
| PGetRotation | 0x45FA70 |
| PResetAccelerationAir | 0x46E630 |
| DamagePlayer | 0x473800 |
| CheckCollision / RunPlayerCollision | 0x485850 / 0x485920 |
| GetCharacterSurfaceInfo | 0x494DF0 |
| HomingAura_Display / LightAttackParticle_DispDelay1 | 0x756520 / 0x757CD0 |

How loading works (my inference from the struct layout, not decompiled): each LoadX allocates an ObjectMaster with MainSub = X_Main and allocates the matching XCharObj2. It copies `PhysicsArray[CharID]` into PhysData, sets ModelList/MotionList, and stores the object in MainCharacter[playerNum]. Each frame, Main runs a switch on EntityData1->Action, then physics (PlayerMovement), collision and animation.

## 3. Characters enum [DOC]
Sonic 0, Shadow 1, Tails 2, Eggman 3, Knuckles 4, Rouge 5, MechTails 6, MechEggman 7, Amy 8, SuperSonic 9, SuperShadow 10, B 11, MetalSonic 12, ChaoWalker 13, DarkChaoWalker 14, Tikal 15, Chaos 16.

## 4. Actions (EntityData1->Action) [DOC]
Shared low range: None 0, Run 1, SpiralLoop 2, SpinCharge 3, SpinRelease 4, Jump 6, Spring 7, SpringLock 8, Launch 9, Fall 10, Balancing 11, Skid 12, HomingAttack 13, Push 14, Pain 15, WallBump 16, SlideGrounded 17, Death 57, Noclip 58.
Values from 59 up overlap. Their meaning depends on which Main function is running:
- 59: LightDash (Sonic) / Glide (Knuckles) / TailsFly
- 60: LightAttack (Sonic, Ancient Light) / MechHover
- Sonic: Somersault1 61, Somersault2 63, SomersaultFinish 64, SomersaultCancel 65, Grind 71, Board 76, BoardBrake 78, BoardFall 79, BoardJump 80, BoardTrick 81, BoardBump 82, GravitySwitch 83
- Knuckles/Rouge: Dig 61, DigFinish 62, DigOnWall 65, DigFinishOnWall 66, DigFailOnWall 68, Climb 69, Punch 71, Hang 72, DrillClaw 78
- Mech: MechPunch 62
- Bounce attack: checked by Sonic_CheckBounceAttack (0x725270) and gated by BounceBracelet.

Status flags (EntityData1->Status): Ground 0x1, OnObjectColli 0x2, Hurt 0x4, ObjectInteract 0x8, Ball 0x100, LightDash 0x200, Attack 0x400, HoldObject 0x800, DoNextAction 0x1000, OnPath 0x2000, DisableControl 0x4000.

## 5. Physics table [DOC]
`PhysicsData PhysicsArray[19]` @ **0x17391E8**, indexed by the Characters enum. Fields in order: int HangTime; float FloorGrip, SpeedCapH, SpeedCapV, SpeedMaxH, PushSpeedMax, JumpSpeed, NoControlSpeed, SlideSpeed, JogSpeed, RunSpeed, RushSpeed, KnockbackSpeed, DashSpeed, JumpAdd, RunAccel, AirAccel, RunDecel, RunBrake, AirBrake, AirDecel, AirResist, AirResistV, AirResistH, GroundFriction, GroundFrictionH, FrictionCap, RatBound, Radius, Height, Weight, CameraY, CenterHeight. That is 33 x 4 bytes = 0x84 per entry. The row is copied into CharObj2Base.PhysData at load, so edits at runtime must target that copy.

## 6. Collision [DOC]
- **CollisionInfo**: u16 Id; s16 HitCount; u16 Flag; u16 Count; float Range; CollisionData* CollisionArray; CollisionHitInfo CollisionHits[16]; Normal; ObjectMaster* Object; my_num; hit_num; CollidingObject.
- **CollisionData**: char kind; char form (shape); char push; char damage; u32 attr; center; param1..4; Rotation.
- Shapes: Sphere 0, Cyl1 1, Cyl2 2, Cube1 3, Cube2 4, Capsule 6, Wall 9.
- Collision lists: Players 0, Projectiles 1, Targetable 2, Enemies 3, Regular 4, Unknown_5 5, Rings 6, Unknown_7 7, Unknown_8 8, Chao 9. Example arrays: CollisionList_4_List @0x1945E18 (511 entries), PlayerCollision_Count @0x1946AB8.
- Damage (partly inferred): the CollisionData `damage` byte and the Status Attack/Ball flags appear to decide which side hurts the other. Related functions: DamagePlayer 0x473800, Sonic_ChecksForDamage 0x719920.
- Terrain: CurrentLandTable @0x19459DC (LandTable*); LandColList @0x1A5A2DC, LandColList_Count @0x1DE9484.
- Dynamic collision: MobileColList @0x1A5A2E0, MobileColList_Count @0x19459D4, DynColQueue @0x1A5A400.
- SurfaceFlags: Solid 0x1, Water 0x2, Diggable 0x20, Unclimbable 0x80, Stairs 0x100, Hurt 0x400, CannotLand 0x1000, Accelerate 0x100000, Dynamic 0x8000000, Visible 0x80000000. Diggable and Unclimbable control where Knuckles/Rouge can dig and climb.

## 7. Upgrades [DOC]
Bitfield `CharObj2Base.Upgrades`:
- Sonic: LightShoes 0x1, AncientLight 0x2, MagicGloves 0x4, FlameRing 0x8, BounceBracelet 0x10, MysticMelody 0x20
- Tails: Booster 0x40, Bazooka 0x80, LaserBlaster 0x100, MysticMelody 0x200
- Knuckles: ShovelClaw 0x400, Sunglasses 0x800, HammerGloves 0x1000, AirNecklace 0x2000, MysticMelody 0x4000
- SuperSonic: 0x8000
- Shadow: AirShoes 0x10000, AncientLight 0x20000, FlameRing 0x40000, MysticMelody 0x80000
- Eggman: JetEngine 0x100000, LargeCannon 0x200000, LaserBlaster 0x400000, ProtectiveArmor 0x800000, MysticMelody 0x1000000
- Rouge: PickNails 0x2000000, TreasureScope 0x4000000, IronBoots 0x8000000, MysticMelody 0x10000000

Save-side "got" bytes run from SonicLightShoesGot @0x1DEB300 to RougeMysticMelodyGot (s16) @0x1DEB31C. Cheats_GiveAllUpgrades is at 0x1A55778.

Which move each upgrade unlocks (based on gameplay knowledge; the gate checks were not traced in code):
- LightShoes: LightDash (action 59)
- AncientLight: LightAttack (action 60)
- FlameRing: breaks metal boxes during the somersault/spin (FlameRingPulse)
- BounceBracelet: bounce attack
- MagicGloves: picks up enemies
- ShovelClaw / PickNails: digging into hard ground
- HammerGloves / IronBoots: breaking boxes
- AirNecklace: unlimited air underwater
- Booster / JetEngine: MechHover
- Bazooka / LargeCannon and LaserBlaster: weapon upgrades
- ProtectiveArmor: MechHP

## 8. Models / motions / skins
Runtime [DOC]:
- `CharacterModels[532]` ModelIndex @0x1DE9620
- SonicAnimList AnimationInfo[203] @0xA17C18
- Each character's CharObj2 holds its own ModelList/MotionList pointers.
- Costumes: `AltCostume[2]` @0x174B013, `AltCharacter` @0x174B015, plus CharObj2Base.Costume.

Files in resource/gd_PC [LOCAL] (PRS-compressed; 1333 files in the folder):
- Sonic: sonicmdl, sonicmtn, sonictex; alt costume: sonic1mdl, sonic1tex
- Shadow: teriosmdl, teriosmtn, teriostex ("Terios" is the internal name for Shadow); alt costume: shadow1mdl, shadow1tex
- Super forms: ssonicmdl/mtn/tex, sshadowmdl/mtn/tex
- Knuckles: knuckmdl, knuckmtn, knucktex; also bknuckmdl, bknucktex
- Rouge: rougemdl, rougemtn, rougetex; also brougemdl, rougeBig.prs
- Tails on foot: milesmdl, milesmtn, milestex; also tailsBig.prs, tails_prison.prs
- Tails mech: twalkmdl/mtn/tex; alt: twalk1mdl/tex
- Eggman mech: ewalkmdl/mtn/tex; also ewalk1mdl/tex and ewalk2mdl/tex
- Chao walkers: cwalkmdl/mtn/tex, dwalkmdl/mtn/tex
- Amy: amymdl, amymtn, amytex, amy_efftex
- Metal Sonic: metalsonicmdl/mtn/tex, metal_2p
- Chaos 0: chaos0mdl/mtn/tex
- Shared motions: plcommtn.prs

What the b*mdl, *Big, ewalk1 and ewalk2 variants are has not been confirmed. The alt-costume and variant model files have no MTN of their own, so presumably they reuse the base character's MTN.

## 9. Character select / swap [DOC]
- CharacterSelection[2] int @0x1A3D8E0
- CharacterSelected[2] @0x1D1B8B2
- CharacterToCostumeIndex[2] @0x1D1B8B0
- CharSelectThings[6] @0x1D1B8C5
- PlayerReady[2] @0x1AEE598
- CharacterSelectTimer @0x1AEE5B0

LoadCharacters (0x43D630) is the dispatcher that calls the per-character LoadX function. Character-swap mods usually hook it or change CurrentCharacter before it runs (common practice, not traced here).

## Verification status
- [LOCAL]: inventory of gd_PC character files.
- [DOC]: all addresses, structs and enums. These were not cross-checked by disassembly; the REA check was skipped.
- SA Mod Manager is noted as the installer/configurator only. It isn't a source for addresses.
