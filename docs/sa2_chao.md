# SA2 Battle (PC) Chao World: Technical Report

Sources:
- SA2 Mod Loader headers. Offsets and structs here are taken verbatim from them.
  - https://github.com/X-Hax/sa2-mod-loader/blob/master/SA2ModLoader/include/SA2Structs.h
  - SA2Enums.h and SA2Variables.h in the same folder.
- CWE modding docs (HTML repo, archived 2024-04-29): https://github.com/NostalgiaNinja/CWEModding
  - Pages read: dllproject, fruitmodding, animals, hatmodding, accessorymodding.
- New docs index: https://nostalgianinja.github.io/ChaoModding_Docs/
  - This is a modding index. Its "Chao Data Structure" subpage URL returned 404.
- Chao Island (https://chao-island.com). The stats page URL I tried returned 404.

Confidence: every **[H]** item comes straight from a header. Every **[K]** item is well-established community knowledge (Chao Island / Sonic Retro) that I did not re-verify in this session.

---

## 1. Local install inventory (read-only)

All paths below are under `D:\SteamLibrary\steamapps\common\Sonic Adventure 2\resource\gd_PC`. There are 192 `al_*` and `Chao*` files.

| Group | Files | Role |
|---|---|---|
| Core | `ChaoMain.prs` (2.54 MB), `ChaoMotionsD.rel` | Chao models and motions (main archive) |
| Stage packs | `ChaoStgNeut/Hero/Dark.prs`, `ChaoStgLobby*.prs` (+`.rel`), `ChaoStgEntrance.prs`, `ChaoStgKinder.prs`, `ChaoStgStadium.prs`, `ChaoStgRace*.prs`, `ChaoStgKarate.prs`, `ChaoStgOdekake.prs` | Per-area geometry and logic data |
| Lobby variants | `000`, `00K`, `0DK`, `H0K`, `HDK` | Lobby versions by unlock state (Hero / Dark garden, Kindergarten) |
| Race data | `ChaoSetRace{Beg,Jwl}{Fly,Pow,Run,Swm,Lng,IL}.bin`, `ChaoSetRaceLand{Hero,Dark,Neut}.bin`, `ChaoRaceCam*.bin` | Course layouts and cameras ("Beg" = beginner, "Jwl" = jewel, "IL" = Challenge) |
| Chao parts | `al_body.prs`, `al_eye.prs`, `al_mouth.prs`, `al_parts.prs`, `al_jewel.prs`, `al_icon.prs`, `al_palette.plt` | Body, eyes, mouth, animal parts, jewel textures, emotion ball, colour palette |
| Animals | `al_mini_parts_tex.prs`, `al_minimal_tex.prs` | Animals ("minimals") |
| Objects | `al_object.prs`, `al_toy.prs`, `al_{neut,hero,dark,lobby,karate}_obj_tex.prs`, `al_tex_stadium_obj.prs` | Garden objects, trees, toys |
| Menus and titles | `al_ent_*`, `al_stagetitle_*_{e,j}_tex`, `al_tex_cave_menu_*` (Black Market cave), `al_tex_odekake_menu_*` (Transporter, "odekake" = going out), `al_karate_adv_tex_*`, `al_race_*` | UI text and title cards |
| GameCube-origin palettes | `al_hXY.GVP`, `al_nc*.GVP`, `al_dc.GVP`, `al_child02.GVP` | Per-evolution palettes: h = hero/dark/neutral body, c/f/n/p/r/s/z = type |

Save data: `resource\gd_PC\SAVEDATA\SONIC2B__S01` is the main save (24,576 bytes). There is no Chao save file (`SONIC2B__ALF`) yet, so Chao World has not been saved on this install.

No `mods` folder exists, so CWE is not installed here.

---

## 2. Memory map [H]

These are SA2Variables.h addresses in `sonic2app.exe`.

| Address | Symbol | Notes |
|---|---|---|
| 0x19F6460 | `ChaoSaveStart` | Start of the in-memory Chao save (the ALF image) |
| 0x19F646C | `ChaoGardensUnlocked` | |
| 0x19F6470 | `ChaoToysUnlocked[3]` | |
| 0x19F6528 | `ChaoFruitSlots[480]` | Garden item instances |
| 0x19F6848 | `ChaoSeedSlots[240]` | Garden item instances |
| 0x19F6938 | `ChaoHatSlots[480]` | Garden item instances |
| 0x19F6B18 | `ChaoAnimalSlots[200]` | Garden item instances |
| 0x19F6EC4 | `ChaoSlots` (`ChaoData[37]`) | Each entry is 0x800 bytes: 0x4DC + 68 + 736 = 2048. The first 24 belong to gardens (8 per garden); the rest are Transporter / lobby slots. |
| 0x174B004 | `HaveChaoKey` | |
| 0x1AEFF64 | `ChaoKeyTaskObj` | |
| 0x1312DE4 | `ChaoTimescale` | `int16` |
| 0x13402AC / 0x134046C / 0x134062C | `LastChaoArea` / `CurrentChaoArea` / `NextChaoArea` | Area IDs from the `ChaoArea` enum |
| 0x1366260 | `ProbablyChaoSpawnPoints[48]` | |
| 0x1366580 | `ChaoSegments[48]` | |
| 0x1367380 | `ChaoHeader` | `LevelHeader` |
| 0x1296C10 | `ChaoKarateOpponents[28]` | `struct_a1`, each embedding two `ChaoData` |
| 0x1DC0FC8 | `ChaoDataArray` | Marked TODO in the header |
| 0x1DC0FFC | `ChaoObjectArray` | Marked TODO in the header |
| 0xC75368 | `ChaoWorldCharacters[6]` | |
| 0x173D06C | `ChaoSaveIndexThing` | |

Light tables also exist for each area: Kindergarten 0x1313670, Stadium 0x1313728, Garden 0x1313CE8, Lobby 0x1313ED0, Transporter 0x1314060, Karate 0x1366DE0, Race 0x1366F60, Health Center 0x8A297C, Black Market 0x8A3638.

## 3. ChaoDataBase layout (0x4DC bytes, packed) [H]

| Off | Field | Notes |
|---|---|---|
| 0x00 | gap[18] | |
| 0x12 | `Name[7]` | Chao-font encoded |
| 0x1A | `GBATexture` | |
| 0x20-0x27 | `Swim/Fly/Run/Power/Stamina/Luck/Intel/Unknown Fraction` | Sub-level progress; drives the level-up meter |
| 0x28 | `StatGrades[8]` | `ChaoGrade`: E=0 .. S=5. CWE adds X. |
| 0x30 | `StatLevels[8]` | 0-99 |
| 0x38 | `int16 StatPoints[8]` | Max 9999 (999 is shown on the DC/GBA side) |
| 0x48-0x7C | ints | Includes the GBA block `someGBAthingstart`..`end` (0x68-0x7C) |
| 0x80 | `ChaoType Type` | |
| 0x81 | `Garden` | `eCHAO_GARDENS` |
| 0x82 | `int16 Happiness` | -100..100 |
| 0x84 | `InKindergarten` | |
| 0x86 | `ClockRollovers` | |
| 0x88 | `AdultClockRollovers` | |
| 0x8A | `Lifespan` | |
| 0x8C | `Lifespan2` | |
| 0x8E | `Reincarnations` | |
| 0x98 | `Seed` | |
| 0xA4 | `TimescaleTimer` | |
| 0xA8 | `float PowerRun` | Negative = Power, positive = Run (per ChaoModding_Docs ChaoDataStruct.md; corrected) |
| 0xAC | `float FlySwim` | Negative = Fly, positive = Swim (per ChaoModding_Docs; corrected) |
| 0xB0 | `float Alignment` | -1 = Dark, 0 = Neutral, +1 = Hero (verified, ChaoModding_Docs) |
| 0xBC | `LastLifeAlignment` | |
| 0xC0 | `float EvolutionProgress` | 0..1, see section 6 |
| 0xD1 | `EyeType`, `MouthType`, `BallType` | `ChaoEyes`, `ChaoMouth`, `ChaoEmotiball` |
| 0xD5 | `Headgear` | `SA2BHat` / `eCHAO_MASK` |
| 0xD6 | `HideFeet` | |
| 0xD7 | `Medal` | `ChaoMedal` 0-15 |
| 0xD8 | `Color` | `SA2BColour` |
| 0xD9 | `MonotoneHighlights` | |
| 0xDA | `Texture` | Jewel / metal |
| 0xDB | `Shiny` | |
| 0xDC | `EggColor` | |
| 0xDD | `BodyType` | Normal / Egg / Omochao / Animal / None |
| 0xDE | `BodyTypeAnimal` | |
| 0x108 | `int16 DoctorMedal` | |
| 0x118 | `int SA2AnimalBehavior` | Bitflags of behaviours learnt from animals |
| 0x11C-0x123 | Animal part types | `SA2BAnimal`, in order: Arm, Ear, Forehead, Horn, Leg, Tail, Wing, Face |
| 0x124 | `ChaoEmotions Emotion` | See section 4 |
| 0x158 | `ClassRoomFlags[4]` | 32-bit `ChaoClassroomLessonFlags` |
| 0x15C | `int16 SA2BToys` | `ChaoToyFlags`: Rattle 0x1 .. WateringCan 0x800 |
| 0x166 | `SA2BCharacterBonds[6]` | `{char a, b; int unknown}` per character: Sonic, Shadow, Tails, Eggman, Knuckles, Rouge |
| 0x1A6 | field[680] | Includes SADX/DC data |
| 0x44E | `ChaoDNA DNA` | See below |

`ChaoDNA` holds paired parent values:
- `ResetTrigger`.
- `<Stat>Grade1/2` for each of the 8 stats (Swim, Fly, Run, Power, Stamina, Luck, Intelligence, Unknown).
- `FavoriteFruit1/2`, `Color1/2`, `MonotoneFlag1/2`, `Texture1/2`, `ShinyFlag1/2`, `EggColor1/2`.

On breeding, each child trait is picked from one of the two DNA alleles of each parent [K].

`ChaoData` is `ChaoDataBase` + 68 bytes + 736 bytes = 0x800. The extra 804 bytes hold unused/SADX data. CWE repurposes part of this padding for its own extended data (CWE stores more in its own save file).

## 4. ChaoEmotions (at +0x124) [H]

| Group | Fields | Notes |
|---|---|---|
| Timers | `Category1Timer`, `SicknessTimer`, `Category2Timer` | int16 |
| Emotions (char) | Joy, Anger, UrgeToCry, Fear, Surprise, Dizziness, Relax, Total | |
| Needs (int16) | Sleepiness, Tiredness, Hunger, MateDesire, Boredom, Lonely, Tire, Stress, Nourish, Conditn, Energy | Typical range 0..10000 [K] |
| Personality (char, -100..100) | Normal_Curiosity, Kindness, CryBaby_Energetic, Naive_Normal, Solitude, Vitality, Glutton, Regain, Skillful, Charm, Chatty, Normal_Carefree, Fickle | |
| Other | `FavoriteFruit` | `ChaoFavouriteFruit` 0-7 |
| Illnesses | Cough, Cold, Rash, RunnyNose, Hiccups, StomachAche level | |

Personality behaviour [K]:
- Personality values are seeded at hatch and drift with treatment.
- Kindness drops when a chao is hit.
- Glutton rises with overeating.

## 5. Key enums [H]

**`ChaoType` (int8):**

| Value | Type |
|---|---|
| 0 | Empty |
| 1 | Egg |
| 2 | Child |
| 3 | Good (unused) |
| 4 | Bad (unused) |
| 5-10 | Neutral: Normal, Swim, Fly, Run, Power, Chaos |
| 11-16 | Hero: same order |
| 17-22 | Dark: same order |
| 0x17 / 0x18 / 0x19 | Tails / Knuckles / Amy character chao |

**`eCHAO_STATS`:** Swm 0, Fly 1, Run 2, Pwr 3, Stm 4, Lck 5, Int 6, Unk 7.

**`ChaoArea` (0-12):** includes Lobby 0, Stadium 9 and NameMachine 12. The remaining values cover the Neutral/Hero/Dark gardens, Race, Karate, Entrance, Kindergarten, Black Market and Transporter. Full values are in SA2Enums.h.

**`eCHAO_GARDENS`:** Neut 1, Hero 2, Dark 3. SA1/DX gardens: SSDX 4, ECDX 5, MRDX 6.

**`SA2BAnimal`:**
- Range: Penguin 0 .. PurpleChaosDrive 0x18.
- Order: Penguin, Seal, Otter, Rabbit, Cheetah, Warthog, Bear, Tiger, Gorilla, Peacock, Parrot, Condor, Skunk, Sheep, Raccoon, HalfFish, SkeletonDog, Bat, Dragon, Unicorn, Phoenix, then the Yellow/Green/Red/Purple Chaos Drives.
- `SA2BAnimalFlags` (int bitmask) records which animals were absorbed.

**`SA2BFruit`:** ChaoGardenFruit 0 .. Grapes 0x17. Includes Hero / Dark / Round / Triangle / Square, Heart, Chao, Smart, Orange, Blue, Pink, Green, Purple, Yellow, Red fruit, Mushroom, Mint Candy and Grapes.

**`ChaoSeed`:** Strong 0, Tasty 1, Hero 2, Dark 3, Round 4, Triangle 5, Square 6.

**`ChaoItemCategory`:** Egg 1, Fruit 3, Seed 7, Hat 9, MenuTheme 0x10.

**Colours and eggs:**
- `SA2BColour`: 0..0xD (Normal, Yellow, White, Brown, SkyBlue, Pink, Blue, Grey, Green, Red, LimeGreen, Purple, Orange, Black).
- `eAL_COLOR` adds INVIS 0xE/0xF and HYPER 0x10.
- `SA2BEggColour` layout:

| Range | Egg kind |
|---|---|
| 0 | Normal |
| 0x01-0x0D | Monotone |
| 0x0E-0x1A | Two-tone |
| 0x1B-0x35 | Shiny variants |
| 0x36 | GlitchyNormal |

- `SADXEggColour` adds jewels plus Pearl 0x41, Metal1/2 0x42/0x43, Glass 0x44.

**Textures and jewels:**
- `SA2BTexture`: 0x1-0xF are jewel/metal textures (Gold, Silver, Ruby, Sapphire, Amethyst, Emerald, Garnet, Aquamarine, Peridot, Topaz, Onyx, Pearl, Metal1/2, Glass), then Moon 0x10.
- `eCHAO_JEWEL` adds DCMetal 0x15/0x16.

**Hats:** `SA2BHat` runs Pumpkin 1 .. GlitchyNormalEggShell 0x46. It covers fixed hats (pumpkin, skull, apple, bucket, pan, flower pot, paper bag, stump, Santa hat, wool cap) and then egg shells by egg colour.

**Behaviours:**
- `eKW_BHV`: ART 0 .. WATCH 0x3E, including DANCE, SING, MUSIC, MINI1-18 (animal-taught) and TOY1-11.
- `eINTENTION`: 0-17.
- `eCHAO_ICON_EMOTE`: MATE, HUNGER, FEAR, ILLNESS.
- `eCHAO_FLAGS`: UseMove 0x2 .. DrawIcon 0x100000.
- `ChaoClassroomLessonFlags`: DrawingLevel1 0x1 .. Tambourine 0x80000000. Covers drawing, singing, dance, instruments, exercise and so on.

## 6. Evolution, alignment and lifecycle

Legend: **VERIFIED** (URL given), **UNVERIFIED** (memory only, treat as approximate).

Sources:
- CI-evo = https://chao-island.com/info-center/life-cycle/evolution.html
- CI-align = https://chao-island.com/info-center/life-cycle/alignment.html
- CI-death = https://chao-island.com/info-center/life-cycle/death.html
- CI-chaos = https://chao-island.com/info-center/advanced/chaos-chao.html
- CND = https://github.com/NostalgiaNinja/ChaoModding_Docs/blob/master/docs/ChaoDataStruct.md

**Evolution:**
- VERIFIED (CI-evo): the chao cocoons after "a few hours". One chao year is about 3 h on modern versions.
- VERIFIED (CI-evo): the adult type comes from **recent** animal/drive interaction, not from stat values. Balanced or no input gives Normal.
- UNVERIFIED: `EvolutionProgress` (+0xC0) reaching 1.0 triggers the cocoon. CND only calls it "Magnitude".
- UNVERIFIED: the per-axis 0.5 cutoff for picking Swim/Fly/Run/Power.

**Axis signs (corrected):**
- VERIFIED (CND): `PowerRun` negative = Power, positive = Run.
- VERIFIED (CND): `FlySwim` negative = Fly, positive = Swim.
- The earlier draft had these signs reversed.

**Alignment:**
- VERIFIED (CND): `Alignment` runs -1 Dark .. 0 .. +1 Hero.
- VERIFIED (CI-align): it moves towards the side of the characters who feed, pet and raise the chao. Hero characters are Sonic, Tails, Knuckles; Dark are Shadow, Rouge, Eggman.
- VERIFIED (CI-align): an opposite-side character hitting the chao shifts it the other way.
- VERIFIED (CI-align): Hero/Dark fruit shift it slightly per fruit.
- UNVERIFIED: the ±0.5 thresholds at evolution. No numeric threshold was found.

**Chaos Chao:**
- VERIFIED (CI-chaos), all of the following:
  - 3rd life or later (2+ reincarnations).
  - Happiness over 80.
  - At least one of every animal given in the current life.
  - Balanced enough that it would otherwise become a Normal type.
- Alignment picks the variant: Neutral gives Light, Hero gives Angel, Dark gives Devil.
- UNVERIFIED: that Chaos Chao never die.

**Lifespan and death:**
- VERIFIED (CI-death): about 5 chao years, about 15 real hours on modern versions. The earlier "about 5 h" was wrong.
- VERIFIED (CI-death): good care extends life; unhappiness and hits shorten it.
- VERIFIED (CI-death): reincarnation needs hidden happiness above 50 (SA2). No bond threshold is documented.
- VERIFIED (CI-death): a pink cocoon means reincarnation; a grey cocoon means the chao disappears.
- VERIFIED (CI-death): name and race badges are kept.
- UNVERIFIED: the default `Lifespan` value of about 3000 and its tick rate.

**Mating:**
- UNVERIFIED: the `MateDesire` mechanics, DNA allele selection and the "about 3 h to adult" figure.

## 7. Stats and grades

Sources:
- CI-stats = https://chao-island.com/info-center/basics/stats.html
- CIW = https://chao-island.com/wiki/Stats. Seen via search snippet only; a direct fetch returned 403.

**Per-level gain:**
- VERIFIED (CIW snippet): gain per level = `grade*3 + 13 ± 2`, uniformly random, with E=0 .. S=5.

| Grade | E | D | C | B | A | S | X (CWE) |
|---|---|---|---|---|---|---|---|
| Points/level | 11-15 | 14-18 | 17-21 | 20-24 | 23-27* | 26-30* | 29-33* (UNVERIFIED) |

*A and S were derived from the formula; the snippet was cut off. The earlier draft table was off by one point and is replaced.*

**Stat progress sources (VERIFIED, CI-stats):**
- Animals and Chaos Drives fill the progress bars by colour group:
  - Swim: yellow
  - Run: green
  - Power: red
  - Fly: purple
- The earlier draft had Green = Fly and Purple = Power, which was wrong.
- Any fruit raises Stamina; the Mushroom (300 rings) gives a large Stamina/Luck boost.
- Luck and Intelligence rise from all animals and drives. Drives raise Luck twice as fast as animals.
- UNVERIFIED: exact fraction amounts per item, and that a drive gives about 4 levels.

**Caps (VERIFIED, CI-stats):**
- Level cap is 99.
- Points cap at 3266 for visible stats and 4000 for Luck/Intelligence on modern versions.
- The field is an int16; CND lists 0-9999 as the allowed field range. The earlier "9999 cap" referred to the field range, not the gameplay cap.
- Dreamcast caps at 999.

**Reincarnation carryover (VERIFIED, CI-stats):** points drop to floor(10%), levels reset to 1, grades are kept.

**Grade inheritance:** UNVERIFIED (DNA struct only).

**Races and karate:** formulas UNVERIFIED. Chao Island has `training-sa2/` pages that I did not read in this pass.

## 8. Animal parts

The 8 part slots at +0x11C-0x123 are set when an animal is absorbed. Which parts appear depends on the animal and on the chao's evolution form. Each part enum value maps to models in `al_parts.prs`.

The behaviour bitfield `SA2AnimalBehavior` (+0x118) unlocks the MINI* behaviours. Examples: penguin skating, seal clapping, gorilla chest beat, skunk spray.

## 9. AI and behaviour

**Task model:**
- Each chao is an `ObjectMaster` (later named `task`). Its `ChaoData1` holds:
  - `ChaoDataBase_ptr` pointing into `ChaoSlots`.
  - `Flags` (`eCHAO_FLAGS`).
  - `MotionTable` and a body-type motion table.
  - `ChaoBehaviourInfo`: current action index/timer, `BehaviourBase[15]` and `BehaviourTimer[15]` (a behaviour stack), think buffers.
  - `NormalModels` / `HeroModels` / `DarkModels` (`ChaoEvos*`).
  - Node positions.

**Decision loop [K, decomp naming `AL_Think*` / `AL_Behavior*`]:**
- Needs (Hunger, Sleepiness, Boredom, Lonely, MateDesire) and Emotions produce candidate intentions (`eINTENTION`).
- Personality values weight the candidates.
- The chosen behaviour (`eKW_BHV`) is pushed onto the behaviour stack, and the motion is played from `ChaoMotionsD.rel` / `ChaoMain`.

**Emotion ball (`al_icon`):**
- `?` appears when the chao is curious.
- `!` appears on surprise.
- A heart appears when it is happy or wants to mate.
- A swirl appears when it is dizzy or sick.
- The ball changes shape for Hero (halo) and Dark (spike) chao.

## 10. Gardens and areas

**Areas:**
- Lobby: `ChaoStgLobby*`, with variants per unlock state.
- Neutral Garden is open by default.
- Hero Garden unlocks with a Hero chao or Hero Chao Key; Dark Garden likewise.
  - The **Chao Key** (`HaveChaoKey` 0x174B004) is found in action stages. It warps the player to the garden mid-stage, and they resume at the checkpoint afterwards.
- Kindergarten:
  - Classroom: lessons set `ClassRoomFlags`.
  - Doctor (Health Center): shows stats and illness; `DoctorMedal`.
  - Principal: sets the clock / timescale. SA2 gives info here.
  - Fortune teller: names the chao (`Name`).
  - Black Market: buys items with rings, using `ChaoItemCategory` items and garden item slots.
- Stadium: Race (Beginner / Jewel / Challenge / Hero / Dark courses, using the `ChaoSetRace*` files) and Karate (Beginner / Standard / Expert / Super, using `ChaoKarateOpponents`).
  - Race speed uses Run/Swim/Fly/Power plus Stamina.
  - Karate HP comes from Stamina; Luck adds a crit/dodge chance.
- Transporter (`ChaoStgOdekake`, "odekake"): GBA Tiny Chao Garden transfer via the GC link, which the PC port stubs out. Also handles moving chao between saves and online (DC).

**Garden items:**
- Trees grow from seeds (`ChaoSeed`) into fruit.
- Garden `ChaoFruitSlots` / `ChaoHatSlots` / `ChaoAnimalSlots` / `ChaoSeedSlots` persist objects and positions.
- `ChaoToysUnlocked` covers toys such as rattle, ball, TV, radio, horse, car, box and watering can.

## 11. How it chains

1. The save (ALF, image at `ChaoSaveStart`) is loaded into `ChaoSlots[]`.
2. On garden entry (`NextChaoArea`), one Chao task is spawned per slot whose `Garden` matches the current area.
3. Every frame the task:
   - decays needs,
   - runs the AI,
   - plays motions,
   - responds to the player (petting changes Alignment, Happiness and bonds; feeding changes stats and Fraction; animals change parts and axes).
4. A timer tick advances `EvolutionProgress` and `Lifespan`. These feed cocoon → evolution (type and models) or reincarnation/death.
5. Stats drive the outcomes in Race and Karate. Medals (`Medal`) and rings feed the Black Market.
6. Kindergarten lessons add behaviours.
7. All of this is written back to `ChaoDataBase` and saved on exit.

## 12. Chao World Extended (CWE)

CWE is a SA2 Mod Loader DLL.

**Hooks and extensions:**
- Hooks the chao load/save, Black Market, fruit, hat and evolution functions.
- Adds X grade and extra evolutions/models.
- Adds more fruit (SADX fruits via DCFruits).
- Adds accessories, a separate save extension and a stats UI.

**Mod API:** other mods get the API via `GetModuleHandle(L"CWE")` + `GetProcAddress` and register content in their `CWELoad()` callback.

| Function | Purpose / notes |
|---|---|
| `RegisterChaoTexlistLoad(name, &texlist)` | Loads the texture `.pak` |
| `RegisterFruit(mdl, buy, sell, stats, func, name, desc)` | Custom fruit IDs start at 34, or 36 with DCFruits |
| `RegisterBlackMarketGeneralFruit(&chance)` | |
| `EasyRegisterHat(mdl, buy, sell, name, desc)` | Hats + accessories are limited to 255 shared |
| `EasyRegisterAccessory(..., EAccessoryType)` | Head, Generic1, Generic2 |
| `RegisterChaoMinimal(CWE_MINIMAL*)` | Custom animal with a `CWE_PARTS` part set |
| `RegisterChaoMinimalFruit(fruitID, minimalID, min%, max%)` | Fruit that spawns that animal |

The docs also cover custom trees, character chao, lenses (eye) and Fortune Teller names.

## Gaps

Status after the verification pass:

**Resolved:**
- The real docs page is https://nostalgianinja.github.io/ChaoModding_Docs/ChaoDataStruct/. Its source is `docs/ChaoDataStruct.md` in the ChaoModding_Docs repo, and it has been used above.

**Still unverified:**
- Evolution and alignment numeric thresholds.
- `Lifespan` constants and tick rate.
- Race and karate formulas.
- Grade X range.
- Mating and DNA selection.
- Sections 9-10 behaviour details (decomp names, emotion-ball shapes) and the Kindergarten/Transporter descriptions.

**Not reachable:**
- chao-island.com/wiki returned 403.
- chao.fandom.com returned 402.
