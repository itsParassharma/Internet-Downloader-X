# SPDX-License-Identifier: MIT
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Chrome', 'Edge', 'Firefox')]
    [string] $Browser,
    [string] $ExtensionId,
    [string] $HostPath = (Join-Path $PSScriptRoot '..\idx-native-host.exe'),
    [switch] $PrepareOnly
)

$ErrorActionPreference = 'Stop'
$idxHostName = 'org.idx.downloader'
if ($Browser -eq 'Firefox') {
    if (-not $ExtensionId) { $ExtensionId = 'idx-downloader@idx.local' }
    if ($ExtensionId -match '[\x00-\x20]' -or $ExtensionId.Length -gt 255) {
        throw 'Supply the Firefox add-on ID from its manifest.'
    }
} elseif ($ExtensionId -cnotmatch '^[a-p]{32}$') {
    throw 'Chrome and Edge require the 32-character extension ID shown on the browser extensions page.'
}

$idxResolvedHost = (Resolve-Path -LiteralPath $HostPath).ProviderPath
if (-not (Test-Path -LiteralPath $idxResolvedHost -PathType Leaf) -or [IO.Path]::GetFileName($idxResolvedHost) -ne 'idx-native-host.exe') {
    throw 'HostPath must point to idx-native-host.exe.'
}
if (-not (Test-Path -LiteralPath (Join-Path ([IO.Path]::GetDirectoryName($idxResolvedHost)) 'idx.exe') -PathType Leaf)) {
    throw 'idx.exe must be in the same directory as idx-native-host.exe.'
}

$idxManifestDirectory = Join-Path $PSScriptRoot 'native-hosts'
$idxManifestPath = [IO.Path]::GetFullPath((Join-Path $idxManifestDirectory ($Browser.ToLowerInvariant() + '.json')))
$idxManifest = [ordered]@{
    name = $idxHostName
    description = 'IDX - Internet Downloader X browser bridge'
    path = $idxResolvedHost
    type = 'stdio'
}
if ($Browser -eq 'Firefox') { $idxManifest.allowed_extensions = @($ExtensionId) }
else { $idxManifest.allowed_origins = @("chrome-extension://$ExtensionId/") }

$idxVendorKey = switch ($Browser) {
    'Chrome' { 'Software\Google\Chrome\NativeMessagingHosts' }
    'Edge' { 'Software\Microsoft\Edge\NativeMessagingHosts' }
    'Firefox' { 'Software\Mozilla\NativeMessagingHosts' }
}
$idxRegistryKey = "$idxVendorKey\$idxHostName"
if ($PSCmdlet.ShouldProcess($idxManifestPath, 'Write native messaging manifest for the selected extension')) {
    [IO.Directory]::CreateDirectory($idxManifestDirectory) | Out-Null
    [IO.File]::WriteAllText($idxManifestPath, ($idxManifest | ConvertTo-Json -Depth 4), (New-Object Text.UTF8Encoding($false)))
    Write-Host "Created $idxManifestPath"
    if (-not $PrepareOnly) {
        if ($PSCmdlet.ShouldProcess("HKCU\$idxRegistryKey", 'Register IDX for this browser and Windows user')) {
            $idxKey = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($idxRegistryKey)
            try { $idxKey.SetValue('', $idxManifestPath, [Microsoft.Win32.RegistryValueKind]::String) }
            finally { $idxKey.Dispose() }
            Write-Host "Connected IDX to $Browser for extension $ExtensionId. Restart the browser if needed."
        }
    }
}
