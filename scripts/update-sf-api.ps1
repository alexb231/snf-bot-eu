[CmdletBinding()]
param(
    [string]$ForkRepoPath = "$env:USERPROFILE\RustroverProjects\sf-api-fork",
    [string]$SnfRepoPath = "",
    [string]$ForkBranch = "snf-main",
    [string]$UpstreamRemote = "upstream",
    [string]$UpstreamUrl = "https://github.com/the-marenga/sf-api.git",
    [switch]$SkipPush,
    [switch]$SkipCargoUpdate,
    [switch]$SkipCargoCheck,
    [switch]$CommitLockfile
)

$ErrorActionPreference = "Stop"

function Resolve-RepoPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PathToResolve,
        [Parameter(Mandatory = $true)]
        [string]$RepoLabel
    )
    if (-not (Test-Path $PathToResolve)) {
        throw "$RepoLabel path does not exist: $PathToResolve"
    }

    $resolved = (Resolve-Path $PathToResolve).Path
    if (-not (Test-Path (Join-Path $resolved ".git"))) {
        throw "$RepoLabel is not a git repository: $resolved"
    }

    return $resolved
}

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RepoPath,
        [Parameter(Mandatory = $true)]
        [string[]]$Args
    )
    Write-Host "git -C $RepoPath $($Args -join ' ')"
    & git -C $RepoPath @Args
    if ($LASTEXITCODE -ne 0) {
        throw "Git command failed in $($RepoPath): git $($Args -join ' ')"
    }
}

function Invoke-CommandInDir {
    param(
        [Parameter(Mandatory = $true)]
        [string]$WorkingDirectory,
        [Parameter(Mandatory = $true)]
        [string]$FilePath,
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )
    Write-Host "$FilePath $($Arguments -join ' ') (cwd: $WorkingDirectory)"
    Push-Location $WorkingDirectory
    try {
        & $FilePath @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "Command failed: $FilePath $($Arguments -join ' ')"
        }
    }
    finally {
        Pop-Location
    }
}

if ([string]::IsNullOrWhiteSpace($SnfRepoPath)) {
    $SnfRepoPath = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

$ForkRepoPath = Resolve-RepoPath -PathToResolve $ForkRepoPath -RepoLabel "Fork repo"
$SnfRepoPath = Resolve-RepoPath -PathToResolve $SnfRepoPath -RepoLabel "SNF repo"
$TauriPath = Join-Path $SnfRepoPath "sfBot\src-tauri"

if (-not (Test-Path $TauriPath)) {
    throw "Could not find src-tauri path: $TauriPath"
}

Write-Host "=== 1) Sync fork branch with upstream/main ==="


$remoteNames = (& git -C $ForkRepoPath remote) 2>$null
if ($LASTEXITCODE -ne 0) {
    throw "Failed to list git remotes in $ForkRepoPath"
}

if ($remoteNames -notcontains $UpstreamRemote) {
    Invoke-Git -RepoPath $ForkRepoPath -Args @("remote", "add", $UpstreamRemote, $UpstreamUrl)
}
else {
    $currentUpstreamUrl = (& git -C $ForkRepoPath remote get-url $UpstreamRemote).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to read URL for remote '$UpstreamRemote' in $ForkRepoPath"
    }
    if ($currentUpstreamUrl -ne $UpstreamUrl) {
        Invoke-Git -RepoPath $ForkRepoPath -Args @("remote", "set-url", $UpstreamRemote, $UpstreamUrl)
    }
}

Invoke-Git -RepoPath $ForkRepoPath -Args @("fetch", $UpstreamRemote)
Invoke-Git -RepoPath $ForkRepoPath -Args @("fetch", "origin")


& git -C $ForkRepoPath rev-parse --verify $ForkBranch *> $null
if ($LASTEXITCODE -eq 0) {
    Invoke-Git -RepoPath $ForkRepoPath -Args @("checkout", $ForkBranch)
}
else {
    & git -C $ForkRepoPath ls-remote --exit-code --heads origin $ForkBranch *> $null
    if ($LASTEXITCODE -eq 0) {
        Invoke-Git -RepoPath $ForkRepoPath -Args @("checkout", "-b", $ForkBranch, "origin/$ForkBranch")
    }
    else {
        Invoke-Git -RepoPath $ForkRepoPath -Args @("checkout", "-b", $ForkBranch)
    }
}

Invoke-Git -RepoPath $ForkRepoPath -Args @("merge", "--no-edit", "$UpstreamRemote/main")

if (-not $SkipPush) {
    Invoke-Git -RepoPath $ForkRepoPath -Args @("push", "origin", $ForkBranch)
}
else {
    Write-Host "Skipping push (SkipPush specified)."
}

Write-Host "=== 2) Refresh SNF to latest fork commit ==="

if (-not $SkipCargoUpdate) {
    Invoke-CommandInDir -WorkingDirectory $TauriPath -FilePath "cargo" -Arguments @("update", "-p", "sf-api")
}
else {
    Write-Host "Skipping cargo update (SkipCargoUpdate specified)."
}

if (-not $SkipCargoCheck) {
    Invoke-CommandInDir -WorkingDirectory $TauriPath -FilePath "cargo" -Arguments @("check")
}
else {
    Write-Host "Skipping cargo check (SkipCargoCheck specified)."
}

if ($CommitLockfile) {
    Write-Host "=== 3) Commit lockfile in SNF repo ==="
    Invoke-Git -RepoPath $SnfRepoPath -Args @("add", "sfBot/src-tauri/Cargo.lock")

    & git -C $SnfRepoPath diff --cached --quiet -- "sfBot/src-tauri/Cargo.lock"
    if ($LASTEXITCODE -eq 0) {
        Write-Host "No Cargo.lock changes staged; skipping commit."
    }
    else {
        Invoke-Git -RepoPath $SnfRepoPath -Args @("commit", "-m", "Update sf-api to latest $ForkBranch")
    }
}

Write-Host "Done."
