# Sonic Adventure DX at D:\SteamLibrary\steamapps\common\Sonic Adventure DX

Legend: **[VERIFIED-LOCAL]** I checked it in this install. **[VERIFIED-WEB]** quoted from the X-Hax header on GitHub. **[RECALL]** from memory of the mod-loader headers or community docs, not re-checked this session. Check [RECALL] items against the headers before relying on them.

## 0. What build is this? [VERIFIED-LOCAL]
- `Sonic Adventure DX.exe` is 90,922,000 bytes. VersionInfo: FileVersion 1.0.0.1, "Copyright (C) 2011", Company SEGA. It imports `steam_api.dll` (SteamAPI_Init, SteamUserStats), d3dx9_43, XINPUT1_3, DINPUT8 and DSOUND. Strings include `AVChrmodels`, `AVSteamManager` and `MOUNTAIN_STEAM`.
- `AppLauncher.exe` (15 MB) is the configuration launcher. Its UI XML files are in `Data/Screens` and `Data/Widgets`.
- `Shader/Sonic*.wpu/.wvu` (FXAA), `SoundData/bgm/wma`, `SoundData/VOICE_JP|VOICE_US|SE`, `Font/*.dds`, `DLC/re-us2.sfd`.
- `win64/`, `linux32/`, `linux64/` and `osx32/` are **empty** directories (Steam depot leftovers). Nothing inside them is ported.
- `install.vdf` writes the registry key HKLM\Software\SEGA\Sonic Adventure DX and copies saves to `%MYDOCS%\SEGA\Sonic Adventure DX`.
- **Conclusion:** this is the **2010/2011 Steam port** (AppID 71250), which came from the PC Games Store/360-era build. It is not the 2004 `sonic.exe`. The game code and CHRMODELS are **statically linked into the 90 MB exe** (hence the `AVChrmodels` string). `system/` has **no DLLs** (no CHRMODELS.DLL, ADV00..03MODELS.DLL or CHAOSTGGARDEN*.DLL) and 1,986 files.
- **Do the 2004 mod-loader addresses apply? NO, not to this exe as installed.** The SA Mod Manager README says: "If you have the Steam version, or a different region, you can convert it to 2004 US." The manager has a `SteamConverter` component that downloads or patches the files back to the 2004 US `sonic.exe`, the DLL set and the PRS/BIN layout. After that conversion, every address below applies. Until then, any address taken from the X-Hax headers is invalid for this exe.
- Data location in this install: `system/` holds the PVM/PVR textures, `*.BIN` and `*.PRS` set and camera files, and the chao text (`CHAODX_MESSAGE_{BLACKMARKET,HINT,ITEM,ODEKAKE,PLAYERACTION,RACE,SYSTEM}_{E,F,G,J,S}.BIN`), plus `CHAO.PVM`, `CHAOBLACKMARKET*.PVM`, `AL_DX_CHAOS.PVM` and others. Models live inside the exe. In the 2004 build they would be in `system/CHRMODELS_orig.dll`, `ADV00MODELS.DLL`..`ADV03MODELS.DLL`, `CHAOSTGGARDEN0{0,1,2}MR/EC/SS*.DLL`, `CHAOSTGRACE.DLL` and similar files.

## 1. Mod-loader reference (2004 US sonic.exe, base 0x400000)
Sources: https://github.com/X-Hax/sadx-mod-loader/tree/master/SADXModLoader/include (SADXStructs.h, SADXFunctions.h, SADXVariables.h, SADXEnums.h, and the newer `sadx-structs`/decomp-style names such as `playertwp`, `playerpwp` and `taskwk`). Decomp-style names: https://github.com/X-Hax/sadx-mod-loader (`include/ninja`, `SADXStructsNew.h`).

### Globals [VERIFIED-WEB, SADXVariables.h]
| Name | Address |
|---|---|
| EntityData1Ptrs[8] (new name: playertwp) | 0x3B42E10 |
| CharObj2Ptrs[8] (playerpwp) | 0x3B3CDF0 |
| PlayerPtrs[8] (ObjectMaster*/task) | 0x3B42E30 |
| PhysicsArray[8] (PhysicsData, `physics_data` / PL_PHYSICS) | 0x9154E8 |
| CurrentCharacter (int16) | 0x3B22DC0 |
| CurrentLevel (int16) | 0x3B22DCC |
| Rings (int16) | 0x3B0F0E4 |
| Lives (char) | 0x3B0EF34 |
| EventFlagArray[512] | 0x3B18808 |
| EventFlagOffset | 0x7E6750 |
| AdventureCompleteFlags[7] | 0x7EF6C4 |
| ChaoSaveFile (char*) | 0x3C88060 |
| ChaoSlots (ChaoData) | 0x3C88878 |
| CrystalRingUpgrade (NJS_OBJECT*) | 0x3C5A940 |

### Character functions [VERIFIED-WEB, SADXFunctions.h]
Sonic_Main 0x49A9B0, Tails_Main 0x461700, Knuckles_Main 0x47A770, Amy_Main 0x48ABF0, Gamma_Main 0x483430, Big_Main 0x490A00.
Sonic_Act1 0x496F50, Sonic_Display 0x4948C0, Sonic_DisplayLightDashModel 0x4A1630.
Super Sonic: Sonic_SuperPhysics_Load/Main/Delete are 0x494350/0x494320/0x4942D0, and Sonic_SuperAura_Load/Main/Display/Delete are 0x55FAF0/0x55E920/0x55E620/0x55E5B0. Super Sonic is still driven by Sonic_Main.
Collision_Init(obj, CollisionData*, count, list) 0x41CAF0, HurtCharacter(char) 0x4506F0, LoadCharacter 0x4157C0, LoadCharacterBoss 0x413840.
Variables: MetalSonicFlag (bool) 0x3B18DB5.
The current header has no `*_RunsActions` names. In it, the per-character action state machine is reached through `X_Main` → `X_Act1/2/3(EntityData1*, EntityData2*, CharObj2*)`.

### Enums [VERIFIED-WEB, SADXEnums.h]
- Characters: Sonic 0, Eggman 1, Tails 2, Knuckles 3, Tikal 4, Amy 5, Gamma 6, Big 7, MetalSonic 8.
- Upgrades. `UpgradeBits_*` are bit indices and `Upgrades_*` = 1<<bit:

| Upgrade | Value | Upgrade | Value |
|---|---|---|---|
| CrystalRing | 0x1 | LongHammer | 0x80 |
| LightShoes | 0x2 | WarriorFeather | 0x100 |
| AncientLight | 0x4 | LaserBlaster | 0x200 |
| JetAnklet | 0x8 | JetBooster | 0x400 |
| RhythmBadge | 0x10 | PowerRod | 0x800 |
| FightingGloves | 0x20 | LifeRing | 0x1000 |
| ShovelClaw | 0x40 | SuperSonic | 0x8000 (bit 15) |

  Per-character masks exist (Upgrades_SonicMask and so on). Big's Lure upgrades are **not** in this enum; they are event flags. Stored in `CharObj2.Upgrades` (short).
  How upgrades gate moves (gameplay knowledge, not from the headers):
  - Sonic: LightShoes enables the light speed dash; CrystalRing shortens the charge; AncientLight enables the Light Speed Attack.
  - Tails: JetAnklet improves flight and speed; RhythmBadge enables the rapid tail attack.
  - Knuckles: ShovelClaw enables digging; FightingGloves enable Maximum Heat.
  - Amy: WarriorFeather enables the spin hammer; LongHammer enlarges the hammer.
  - Gamma: JetBooster enables hover; LaserBlaster raises the lock-on count.
  - Big: LifeRing keeps him from sinking; PowerRod makes reeling stronger.
- LevelIDs: StationSquare 26, EggCarrierOutside 29, EggCarrierInside 32, MysticRuins 33, Past 34, **SSGarden 39, ECGarden 40, MRGarden 41, ChaoRace 42**.

### Structs [VERIFIED-WEB, SADXStructs.h; field order as declared]
- **EntityData1**: char Action, NextAction, Unknown, Index; short Status, InvulnerableTime, field_A; char CharIndex, CharID; NJS_OBJECT* Object; Loop* LoopData; Rotation3 Rotation; NJS_VECTOR Position, Scale; CollisionInfo* CollisionInfo; void* field_3C.
- **CharObj2**: float SpindashSpeed; short Upgrades, Powerups, JumpTime, NoControlTime, UnderwaterTime, IdleTime; ... NJS_VECTOR Speed, SurfaceNormal ...; int SurfaceFlags, SurfaceFlags_Old; ObjectMaster* ObjectHeld, DynColObject; float LoopDist, Up, TailsFlightTime; short LightdashTime/Timer, SonicSpinTimer; **PhysicsData_t PhysicsData** (embedded copy); AnimThing AnimationThing; NJS_VECTOR SoManyVectors[12]; struct_a3. This list is condensed; see the header for exact ordering.
- **PhysicsData_t** (PhysicsArray[8] at 0x9154E8): int HangTime; float FloorGrip, HSpeedCap, VSpeedCap, MaxAccel, field_14, JumpSpeed, SpringControl, field_20, RollCancel, RollEnd, Run1, Knockback, Run2, JumpAddSpeed, GroundAccel, AirAccel, GroundDecel, Brake, AirBrake, AirDecel, RollDecel, GravityAdd, HitSpeed, MinSpeed, field_64, field_68, field_6C, RippleSize, CollisionSize, Gravity, CameraY, YOff. That is 1 int + 32 floats = 0x84 bytes.
- **CollisionData** (CCL_INFO): char kind, form, push, damage; Uint32 attr; NJS_VECTOR center; float a, b, c, d; int angx, angy, angz.
- **CollisionInfo** (colliwk): u16 id, flag, nbInfo; s16 nbHit, my_num, hit_num; float colli_range; CollisionData* CollisionArray; CollisionThing CollisionThings[16]; NJS_POINT3 normal; ObjectMaster* Object; CollisionInfo* CollidingObject.

### Models and skins
- 2004 build: `system/CHRMODELS_orig.dll` holds the models. The mod loader's `CHRMODELS.dll` proxy forwards the exports, and mods replace models and textures through the HelperFunctions and replace-file APIs.
- Metal Sonic: Sonic's controller with MetalSonicFlag (0x3B18DB5) set.
- Dreamcast look: PkR's Dreamcast Characters/Conversion mods (https://github.com/PiKeyAr/sadx_dreamcast).
- This Steam exe has CHRMODELS compiled in (`AVChrmodels`), so there is no DLL to swap.

## 2. Chao in SADX
- Gardens: Station Square (inside the hotel, which also has the **Chao Race** entrance), Egg Carrier (cabin), Mystic Ruins (jungle hilltop). Each garden holds up to 8 chao (24 in total). Level IDs: SSGarden 39, ECGarden 40, MRGarden 41, ChaoRace 42 [VERIFIED-WEB]. Functions [VERIFIED-WEB]: ChaoGardenFunc_BlackMarket 0x718BF0, ChaoGardenFunc_RaceEntry 0x7199E0, ChaoStgRace_Main 0x719CA0, LoadChaoRace 0x719DB0, DrawBlackMarketItemsList 0x7270E0.
- **Black Market** (`CHAOBLACKMARKET*.PVM`, `CHAODX_MESSAGE_BLACKMARKET_*.BIN`) is DX-only. You spend rings banked by playing to buy eggs, fruit, hats and similar items. It is reached from the Station Square garden area. SADX has no Chao Stadium or Kindergarten; those are SA2.
- Chao Race: a DX-only overhaul of the DC version. It has the Pearl, Amethyst, Sapphire, Ruby and Emerald courses plus Jewel races.
- **ChaoData** [VERIFIED-WEB]: `ChaoData { ChaoDataBase data; char field_520[736]; }` = 0x800 bytes per slot. ChaoSlots is at 0x3C88878. ChaoDataBase offsets, computed from the declaration:

| Offset | Field |
|---|---|
| 0x12 | Name[7] |
| 0x20 | Swim/Fly/Run/Power/Stamina/Lucky/Intelligence/Unknown **Fraction** |
| 0x28 | ...**Grade** |
| 0x30 | ...**Level** |
| 0x38 | Swim/Fly/Run/Power/Stamina/Luck **Stat** (short), then Intelligence/Unknown (char) |
| 0x80 | `ChaoType Type` (int8) |
| 0x81 | Garden |
| 0x82 | Happiness |
| 0x86 | ClockRollovers |
| 0x8A | Lifespan |
| 0x8C | Lifespan2 |
| 0x8E | Reincarnations |
| 0xA8 | float PowerRun |
| 0xAC | float FlySwim |
| 0xB0 | float Alignment |
| 0xC0 | float EvolutionProgress |
| 0xD1 | EyeType, MouthType, BallType, (gap), Headgear, HideFeet, Medal, Color, MonotoneHighlights, Texture, Shiny, EggColor |
| 0xDD | SADXBodyType BodyType |
| 0xDE | BodyTypeAnimal |
| ~0x118 | int SA2AnimalBehavior, then SA2BAnimal Arm/Ear/Forehead/Horn/Leg/Tail/Wing/Face types, Joy, ... |

  Hand-check the tail offsets against the header.
- ChaoType enum: Empty 0, Egg 1, Child 2, Good 3 and Bad 4 (both unobtainable), Neutral/Hero/Dark × Normal 5-7, Swim 8-10, Fly 11-13, Run 14-16, Power 17-19, Chaos 20-22, Tails 23, Knuckles 24, Amy 25.
- Evolution (gameplay): in SA1/SADX, the evolved form comes from the dominant stat and from the small-animal parts the chao picks up. The `SA2BAnimal` fields hold SA2-style parts kept for transfer compatibility.
- **Correction** to the earlier draft: SADX has no in-game hero/dark alignment mechanic, so "alignment by character" does not apply. Hero and Dark types exist in the save format because chao can arrive from SA2B on GameCube.
- Character chao (Tails/Knuckles/Amy, types 23-25) are DX features. The headers do not say what triggers them; my understanding is that it depends on how much the chao likes each character, but I did not confirm that from a source.
- **GBA Tiny Chao Garden**: on the GameCube release, Chao transfer goes over the link cable (the `ODEKAKE` messages = "go out"). The PC and Steam versions do not have it. Mods restore similar features.
- Differences from SA2: SADX has fewer chao types (no SA2 character chao) and no stadium karate. DX has the Black Market. Chao can be transferred to SA2B on GameCube only. Fruit uses the DX garden fruit trees. Comparison docs: https://nostalgianinja.github.io/ChaoModding_Docs/ (mostly SA2B; its struct offsets match SADX's ChaoDataBase for most fields).

## Sources
- https://github.com/X-Hax/sadx-mod-loader (include/SADXVariables.h, quoted above)
- https://github.com/X-Hax/SA-Mod-Manager (README on converting Steam to 2004 US, `SteamConverter`)
- https://info.sonicretro.org/Sonic_Adventure_DX:_Director%27s_Cut
- https://nostalgianinja.github.io/ChaoModding_Docs/
