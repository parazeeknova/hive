param(
    [string]$Executable = (Join-Path $env:USERPROFILE '.cargo\bin\hive-local.exe'),
    [string]$Config = (Join-Path $env:APPDATA 'hive-local\config.json')
)
$ErrorActionPreference = 'Stop'
$Executable = (Resolve-Path -LiteralPath $Executable).Path
$Config = (Resolve-Path -LiteralPath $Config).Path
$User = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
$Action = New-ScheduledTaskAction -Execute $Executable -Argument ('run --config "' + $Config + '"')
$Trigger = New-ScheduledTaskTrigger -AtLogOn -User $User
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1) -ExecutionTimeLimit ([TimeSpan]::Zero)
$Principal = New-ScheduledTaskPrincipal -UserId $User -LogonType Interactive -RunLevel Limited
Register-ScheduledTask -TaskName 'Hive Local' -Action $Action -Trigger $Trigger -Settings $Settings -Principal $Principal -Description 'Serve approved local MCP tools through the hosted Hive endpoint' -Force | Out-Null
Start-ScheduledTask -TaskName 'Hive Local'
Write-Output 'Hive Local starts now and at sign-in. Remove it with Unregister-ScheduledTask -TaskName "Hive Local".'
