# ? A shell started from WSL may be older than the Rust install's PATH change, so we read where Rust
# ? is from the user variables rustup set.
function Use-Rust {
    $cargoHome = [Environment]::GetEnvironmentVariable('CARGO_HOME', 'User')
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue) -and $cargoHome) {
        $env:CARGO_HOME = $cargoHome
        $env:RUSTUP_HOME = [Environment]::GetEnvironmentVariable('RUSTUP_HOME', 'User')
        $env:Path = (Join-Path $cargoHome 'bin') + ';' + $env:Path
    }
}

# - Lists each missing tool and where to get it, then stops before any work starts.
function Assert-Prerequisites([switch] $Clippy, [switch] $Arm64) {
    $missing = @()
    # ? rustup reports a first-use install of rust-toolchain.toml on stderr.
    $ErrorActionPreference = 'Continue'
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        $missing += 'Rust: install rustup from https://rustup.rs, then run rustup toolchain install in the repository'
    } elseif ($Clippy -and -not ((& rustup component list --installed 2>$null) -match '^clippy')) {
        $missing += 'Rust clippy: rustup component add clippy'
    }
    if (-not (Get-Command dotnet -ErrorAction SilentlyContinue) -or -not ((& dotnet --list-sdks) -match '^10\.')) {
        $missing += '.NET 10 SDK: https://dotnet.microsoft.com/download/dotnet/10.0'
    }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    $vc = if (Test-Path -LiteralPath $vswhere) {
        & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    }
    if (-not $vc) {
        $missing += 'MSVC build tools ("Desktop development with C++"): https://visualstudio.microsoft.com/visual-cpp-build-tools/'
    }
    if ($Arm64) {
        $arm = if (Test-Path -LiteralPath $vswhere) {
            & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64 -property installationPath
        }
        if (-not $arm) {
            $missing += 'MSVC ARM64 build tools: add "MSVC ARM64 build tools" in the Visual Studio Installer'
        }
        # - ring builds its ARM64 code with clang.
        if (-not (Get-Command clang -ErrorAction SilentlyContinue)) {
            $missing += 'clang for aarch64: add "C++ Clang tools for Windows" in the Visual Studio Installer and put its bin on PATH'
        }
        if (-not ((& rustup target list --installed 2>$null) -match '^aarch64-pc-windows-msvc$')) {
            $missing += 'Rust target aarch64-pc-windows-msvc: rustup target add aarch64-pc-windows-msvc'
        }
    }
    if ($missing) {
        Write-Host 'Missing prerequisites:'
        $missing | ForEach-Object { Write-Host "  - $_" }
        exit 1
    }
}

function Invoke-Checked([string] $program, [string[]] $arguments) {
    & $program @arguments
    if ($LASTEXITCODE -ne 0) { throw "$program $($arguments -join ' ') failed ($LASTEXITCODE)" }
}
