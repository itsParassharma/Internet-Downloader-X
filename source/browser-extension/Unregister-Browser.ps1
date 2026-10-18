# SPDX-License-Identifier: MIT
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('Chrome', 'Edge', 'Firefox')]
    [string] $Browser
)

$ErrorActionPreference = 'Stop'
$idxHostName = 'org.idx.downloader'
$idxManifestPath = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ('native-hosts\' + $Browser.ToLowerInvariant() + '.json')))
$idxVendorKey = switch ($Browser) {
    'Chrome' { 'Software\Google\Chrome\NativeMessagingHosts' }
    'Edge' { 'Software\Microsoft\Edge\NativeMessagingHosts' }
    'Firefox' { 'Software\Mozilla\NativeMessagingHosts' }
}
$idxRegistryKey = "$idxVendorKey\$idxHostName"
$idxKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($idxRegistryKey)
$idxRegisteredPath = $null
if ($idxKey) {
    try { $idxRegisteredPath = $idxKey.GetValue('') }
    finally { $idxKey.Dispose() }
}
if ($idxRegisteredPath -and -not [StringComparer]::OrdinalIgnoreCase.Equals($idxRegisteredPath, $idxManifestPath)) {
    throw 'This browser is connected to another IDX folder. Run its unregister script instead.'
}
if ($idxRegisteredPath -and $PSCmdlet.ShouldProcess("HKCU\$idxRegistryKey", 'Remove this IDX browser connection')) {
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKey($idxRegistryKey, $false)
}
if ((Test-Path -LiteralPath $idxManifestPath -PathType Leaf) -and $PSCmdlet.ShouldProcess($idxManifestPath, 'Remove the generated native host manifest')) {
    Remove-Item -LiteralPath $idxManifestPath
}
Write-Host "Unregister step complete for $Browser. Remove the browser extension separately if desired."
