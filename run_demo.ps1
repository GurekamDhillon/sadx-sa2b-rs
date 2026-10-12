# Launches the Sonic demo: release build, Vulkan, vsync off (uncapped fps).
# Add -Vsync to cap at your monitor's refresh rate.
param([switch]$Vsync)
$env:SA2_DIR = "D:\SteamLibrary\steamapps\common\Sonic Adventure 2"
if ($Vsync) { Remove-Item Env:DEMO_NOVSYNC -ErrorAction SilentlyContinue } else { $env:DEMO_NOVSYNC = "1" }
Set-Location $PSScriptRoot
cargo run --release -p sa_demo
