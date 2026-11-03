param(
    [Parameter(Mandatory)][string]$Config,
    [Parameter(Mandatory)][string]$PythonWindowless
)
$ErrorActionPreference = 'Stop'
$taskName = 'AzerothOperator-ExportSync'
if (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue) {
    throw 'Existing task: inspect and reconcile it before replacing'
}
foreach ($path in @($Config, $PythonWindowless)) {
    if (-not [IO.Path]::IsPathFullyQualified($path) -or -not (Test-Path -LiteralPath $path -PathType Leaf) -or $path.Contains('"')) {
        throw 'Expected existing absolute file path without quotes'
    }
}
if ([IO.Path]::GetFileName($PythonWindowless) -ne 'pythonw.exe') {
    throw 'Windowless Python is required'
}
$settingsConfig = Get-Content -LiteralPath $Config -Raw | ConvertFrom-Json
if (-not [IO.Path]::IsPathFullyQualified($settingsConfig.state)) { throw 'Absolute state directory required' }
$source = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\tools\sync_exports.py'))
$hash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
$runtime = Join-Path $settingsConfig.state "runtime\$hash"
New-Item -ItemType Directory -Path $runtime -Force | Out-Null
$script = Join-Path $runtime 'sync_exports.py'
Copy-Item -LiteralPath $source -Destination $script
if ((Get-FileHash -LiteralPath $script -Algorithm SHA256).Hash.ToLowerInvariant() -ne $hash) { throw 'Runner copy mismatch' }
$identity = [Security.Principal.WindowsIdentity]::GetCurrent().Name
$action = New-ScheduledTaskAction -Execute $PythonWindowless -Argument "`"$script`" --config `"$Config`"" -WorkingDirectory $runtime
$triggers = @(
    (New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(1) -RepetitionInterval (New-TimeSpan -Minutes 5)),
    (New-ScheduledTaskTrigger -AtLogOn -User $identity)
)
$principal = New-ScheduledTaskPrincipal -UserId $identity -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Minutes 10) -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $triggers -Principal $principal -Settings $settings -Description 'Upload saved Azeroth Operator exports; no interaction with WoW.' | Out-Null
Start-ScheduledTask -TaskName $taskName
[pscustomobject]@{TaskName=$taskName; RunnerSHA256=$hash; RunLevel='Limited'; IntervalMinutes=5}
