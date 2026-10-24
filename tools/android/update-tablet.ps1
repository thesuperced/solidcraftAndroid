param([string]$Serial)
$ErrorActionPreference = "Stop"
$adb = Join-Path $env:LOCALAPPDATA "Android\Sdk\platform-tools\adb.exe"
if (-not (Test-Path $adb)) { throw "ADB not found at $adb" }

if (-not $Serial) {
    $devices = @(& $adb devices | ForEach-Object {
        $match = [regex]::Match($_, '^([^\s]+)\s+device$')
        if ($match.Success) { $match.Groups[1].Value }
    })
    if ($devices.Count -ne 1) { throw "Connect and authorize one tablet, or pass -Serial YOUR_DEVICE_ID." }
    $Serial = $devices[0]
}

$apk = Join-Path $env:USERPROFILE "Downloads\solidcraft-pen-first-arm64.apk"
$url = "https://github.com/thesuperced/solidcraftAndroid/raw/refs/heads/feat/android-pen-touch-keypad/artifacts/android/solidcraft-pen-first-arm64.apk"
Invoke-WebRequest -Uri $url -OutFile $apk
$expected = "837f5965173128de3d47a0e50349771e105b9f0692127966ea3c7d1c81e7bfdd"
if ((Get-FileHash -Algorithm SHA256 $apk).Hash -ine $expected) { throw "APK checksum mismatch; installation stopped." }

function Invoke-Adb {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$AdbArguments)
    & $adb -s $Serial @AdbArguments
    if ($LASTEXITCODE -ne 0) { throw "ADB failed: $AdbArguments" }
}
Invoke-Adb -AdbArguments @("shell", "am", "force-stop", "ai.storyteller.solidcraft.pentouch")
Invoke-Adb -AdbArguments @("install", "-r", $apk)
Invoke-Adb -AdbArguments @("shell", "am", "start", "-n", "ai.storyteller.solidcraft.pentouch/ai.storyteller.solidcraft.SolidCraftActivity")
Write-Host "Updated and launched SolidCraft PenTouch Test; app data retained."
