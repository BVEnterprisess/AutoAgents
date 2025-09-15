# AutoAgents Build Issues Fix - Automated Build Optimization and Dependency Resolution
# This script resolves common build issues and optimizes the build process

param(
    [switch]$InstallRust,
    [switch]$InstallWasmTarget,
    [switch]$UpdateDependencies,
    [switch]$CleanBuild,
    [switch]$OptimizeBuild,
    [string]$ProjectPath = $PSScriptRoot,
    [switch]$Verbose
)

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $color = switch ($Level) {
        "ERROR" { "Red" }
        "WARNING" { "Yellow" }
        "SUCCESS" { "Green" }
        default { "White" }
    }
    Write-Host "[$timestamp] [$Level] $Message" -ForegroundColor $color
}

function Test-RustInstallation {
    try {
        $rustVersion = & rustc --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Log "Rust is installed: $rustVersion" "SUCCESS"
            return $true
        }
    } catch {
        Write-Log "Rust is not installed or not in PATH" "WARNING"
    }
    return $false
}

function Install-Rust {
    Write-Log "Installing Rust using rustup-init.exe..." "INFO"

    $rustupPath = Join-Path $PSScriptRoot "rustup-init.exe"
    if (!(Test-Path $rustupPath)) {
        Write-Log "rustup-init.exe not found in project directory" "ERROR"
        Write-Log "Please download rustup-init.exe from https://rustup.rs/" "WARNING"
        return $false
    }

    try {
        Write-Log "Running Rust installer..." "INFO"
        $installProcess = Start-Process -FilePath $rustupPath -ArgumentList "--default-toolchain", "stable", "--profile", "default", "-y" -Wait -PassThru

        if ($installProcess.ExitCode -eq 0) {
            Write-Log "Rust installation completed successfully" "SUCCESS"

            # Refresh environment variables
            $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")

            # Test installation
            if (Test-RustInstallation) {
                return $true
            } else {
                Write-Log "Rust installation verification failed" "ERROR"
                return $false
            }
        } else {
            Write-Log "Rust installation failed with exit code: $($installProcess.ExitCode)" "ERROR"
            return $false
        }
    } catch {
        Write-Log "Error during Rust installation: $($_.Exception.Message)" "ERROR"
        return $false
    }
}

function Install-WasmTarget {
    Write-Log "Installing wasm32-wasip1 target..." "INFO"

    try {
        $result = & rustup target add wasm32-wasip1 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Log "wasm32-wasip1 target installed successfully" "SUCCESS"
            return $true
        } else {
            Write-Log "Failed to install wasm32-wasip1 target: $result" "ERROR"
            return $false
        }
    } catch {
        Write-Log "Error installing WASM target: $($_.Exception.Message)" "ERROR"
        return $false
    }
}

function Update-CargoDependencies {
    param([string]$Path)

    Write-Log "Updating Cargo dependencies..." "INFO"

    Push-Location $Path
    try {
        # Update Cargo.lock
        Write-Log "Updating Cargo.lock..." "INFO"
        $updateResult = & cargo update 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Log "Cargo dependencies updated successfully" "SUCCESS"
        } else {
            Write-Log "Cargo update failed: $updateResult" "ERROR"
            return $false
        }

        # Check for outdated dependencies
        Write-Log "Checking for outdated dependencies..." "INFO"
        $outdatedResult = & cargo outdated 2>&1
        if ($LASTEXITCODE -eq 0) {
            if ($outdatedResult -match "All dependencies are up to date") {
                Write-Log "All dependencies are up to date" "SUCCESS"
            } else {
                Write-Log "Some dependencies are outdated. Consider updating them manually." "WARNING"
                if ($Verbose) {
                    Write-Host $outdatedResult
                }
            }
        }

        return $true
    } catch {
        Write-Log "Error updating dependencies: $($_.Exception.Message)" "ERROR"
        return $false
    } finally {
        Pop-Location
    }
}

function Clean-BuildArtifacts {
    param([string]$Path)

    Write-Log "Cleaning build artifacts..." "INFO"

    Push-Location $Path
    try {
        # Clean Cargo build artifacts
        Write-Log "Cleaning Cargo build cache..." "INFO"
        $cleanResult = & cargo clean 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Log "Cargo build cache cleaned" "SUCCESS"
        } else {
            Write-Log "Cargo clean failed: $cleanResult" "WARNING"
        }

        # Remove target directories
        $targetDirs = Get-ChildItem -Path . -Directory -Recurse | Where-Object { $_.Name -eq "target" }
        foreach ($dir in $targetDirs) {
            try {
                Remove-Item -Path $dir.FullName -Recurse -Force
                Write-Log "Removed target directory: $($dir.FullName)" "SUCCESS"
            } catch {
                Write-Log "Failed to remove target directory $($dir.FullName): $($_.Exception.Message)" "WARNING"
            }
        }

        # Clean node_modules if present
        if (Test-Path "node_modules") {
            Write-Log "Cleaning node_modules..." "INFO"
            try {
                Remove-Item -Path "node_modules" -Recurse -Force
                Write-Log "node_modules cleaned" "SUCCESS"
            } catch {
                Write-Log "Failed to clean node_modules: $($_.Exception.Message)" "WARNING"
            }
        }

        # Clean other common build artifacts
        $artifactsToClean = @(
            "*.exe", "*.dll", "*.lib", "*.pdb",  # Windows binaries
            "*.o", "*.a", "*.so", "*.dylib",     # Unix binaries
            ".DS_Store",                        # macOS
            "Thumbs.db"                         # Windows
        )

        foreach ($pattern in $artifactsToClean) {
            $files = Get-ChildItem -Path . -File -Recurse -Filter $pattern -ErrorAction SilentlyContinue
            if ($files) {
                $files | Remove-Item -Force
                Write-Log "Cleaned $($files.Count) $pattern files" "SUCCESS"
            }
        }

        return $true
    } catch {
        Write-Log "Error during cleanup: $($_.Exception.Message)" "ERROR"
        return $false
    } finally {
        Pop-Location
    }
}

function Optimize-BuildConfiguration {
    param([string]$Path)

    Write-Log "Optimizing build configuration..." "INFO"

    Push-Location $Path
    try {
        # Check if Cargo.toml exists
        if (!(Test-Path "Cargo.toml")) {
            Write-Log "Cargo.toml not found, skipping build optimization" "WARNING"
            return $false
        }

        # Read current Cargo.toml
        $cargoToml = Get-Content "Cargo.toml" -Raw

        # Add optimization profiles if not present
        if ($cargoToml -notmatch "\[profile\.release\]") {
            Write-Log "Adding release profile optimizations..." "INFO"
            $optimizationConfig = @"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
"@

            $cargoToml += $optimizationConfig
            $cargoToml | Set-Content "Cargo.toml"
            Write-Log "Release profile optimizations added" "SUCCESS"
        }

        # Add build cache configuration
        if ($cargoToml -notmatch "\[build\]") {
            Write-Log "Adding build cache configuration..." "INFO"
            $buildConfig = @"

[build]
rustflags = ["-C", "target-cpu=native"]
"@

            $cargoToml += $buildConfig
            $cargoToml | Set-Content "Cargo.toml"
            Write-Log "Build cache configuration added" "SUCCESS"
        }

        # Create .cargo/config.toml for additional optimizations
        $cargoDir = ".cargo"
        $configPath = Join-Path $cargoDir "config.toml"

        if (!(Test-Path $cargoDir)) {
            New-Item -ItemType Directory -Path $cargoDir | Out-Null
        }

        $configContent = @"
[build]
rustflags = ["-C", "target-cpu=native", "-C", "target-feature=+avx2", "-C", "opt-level=3"]

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
opt-level = 0
debug = true
"@

        $configContent | Set-Content $configPath
        Write-Log "Cargo configuration optimized" "SUCCESS"

        return $true
    } catch {
        Write-Log "Error optimizing build configuration: $($_.Exception.Message)" "ERROR"
        return $false
    } finally {
        Pop-Location
    }
}

function Test-Build {
    param([string]$Path)

    Write-Log "Testing build process..." "INFO"

    Push-Location $Path
    try {
        # Check if this is a Cargo project
        if (Test-Path "Cargo.toml") {
            Write-Log "Detected Cargo project, testing build..." "INFO"

            # Test debug build
            Write-Log "Testing debug build..." "INFO"
            $debugResult = & cargo check 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Log "Debug build check passed" "SUCCESS"
            } else {
                Write-Log "Debug build check failed:" "ERROR"
                if ($Verbose) {
                    Write-Host $debugResult
                }
                return $false
            }

            # Test release build
            Write-Log "Testing release build..." "INFO"
            $releaseResult = & cargo build --release 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Log "Release build passed" "SUCCESS"
                return $true
            } else {
                Write-Log "Release build failed:" "ERROR"
                if ($Verbose) {
                    Write-Host $releaseResult
                }
                return $false
            }
        } else {
            Write-Log "No supported build system detected" "WARNING"
            return $false
        }
    } catch {
        Write-Log "Error during build test: $($_.Exception.Message)" "ERROR"
        return $false
    } finally {
        Pop-Location
    }
}

function Show-BuildStatus {
    param([string]$Path)

    Write-Log "Build Status Report:" "INFO"
    Write-Host "=" * 50 -ForegroundColor Cyan

    Push-Location $Path
    try {
        # Check Rust installation
        $rustInstalled = Test-RustInstallation
        Write-Host "Rust Installation: $(if ($rustInstalled) { "✅ Installed" } else { "❌ Not Installed" })" -ForegroundColor $(if ($rustInstalled) { "Green" } else { "Red" })

        # Check WASM target
        if ($rustInstalled) {
            $wasmTarget = & rustup target list --installed 2>$null | Select-String "wasm32-wasip1"
            $wasmInstalled = $wasmTarget -ne $null
            Write-Host "WASM Target: $(if ($wasmInstalled) { "✅ Installed" } else { "❌ Not Installed" })" -ForegroundColor $(if ($wasmInstalled) { "Green" } else { "Red" })
        }

        # Check project structure
        $hasCargoToml = Test-Path "Cargo.toml"
        Write-Host "Cargo.toml: $(if ($hasCargoToml) { "✅ Found" } else { "❌ Not Found" })" -ForegroundColor $(if ($hasCargoToml) { "Green" } else { "Red" })

        if ($hasCargoToml) {
            $hasCargoLock = Test-Path "Cargo.lock"
            Write-Host "Cargo.lock: $(if ($hasCargoLock) { "✅ Found" } else { "❌ Not Found" })" -ForegroundColor $(if ($hasCargoLock) { "Green" } else { "Red" })

            # Check target directory
            $hasTargetDir = Test-Path "target"
            Write-Host "Target Directory: $(if ($hasTargetDir) { "✅ Exists" } else { "❌ Not Found" })" -ForegroundColor $(if (!$hasTargetDir) { "Green" } else { "Yellow" })
        }

        # Check disk space
        $disk = Get-WmiObject -Class Win32_LogicalDisk -Filter "DeviceID='C:'"
        $freeSpaceGB = [math]::Round($disk.FreeSpace / 1GB, 2)
        $diskStatus = if ($freeSpaceGB -gt 10) { "✅ Sufficient" } elseif ($freeSpaceGB -gt 5) { "⚠️ Limited" } else { "❌ Low" }
        Write-Host "Free Disk Space: ${freeSpaceGB}GB $diskStatus" -ForegroundColor $(if ($freeSpaceGB -gt 10) { "Green" } elseif ($freeSpaceGB -gt 5) { "Yellow" } else { "Red" })

    } catch {
        Write-Log "Error generating build status report: $($_.Exception.Message)" "ERROR"
    } finally {
        Pop-Location
    }

    Write-Host "=" * 50 -ForegroundColor Cyan
}

# Main execution
try {
    Write-Host "🔧 AutoAgents Build Issues Fix" -ForegroundColor Green
    Write-Host "=" * 35 -ForegroundColor Green
    Write-Host ""

    # Show initial build status
    Show-BuildStatus -Path $ProjectPath

    $fixesApplied = 0

    # Install Rust if requested
    if ($InstallRust) {
        if (Install-Rust) {
            $fixesApplied++
        }
    }

    # Install WASM target if requested
    if ($InstallWasmTarget) {
        if (Install-WasmTarget) {
            $fixesApplied++
        }
    }

    # Clean build artifacts if requested
    if ($CleanBuild) {
        if (Clean-BuildArtifacts -Path $ProjectPath) {
            $fixesApplied++
        }
    }

    # Update dependencies if requested
    if ($UpdateDependencies) {
        if (Update-CargoDependencies -Path $ProjectPath) {
            $fixesApplied++
        }
    }

    # Optimize build configuration if requested
    if ($OptimizeBuild) {
        if (Optimize-BuildConfiguration -Path $ProjectPath) {
            $fixesApplied++
        }
    }

    # If no specific fixes requested, apply all
    if (!$InstallRust -and !$InstallWasmTarget -and !$CleanBuild -and !$UpdateDependencies -and !$OptimizeBuild) {
        Write-Log "No specific fixes requested, applying comprehensive fix..." "INFO"

        # Check and install Rust if needed
        if (!(Test-RustInstallation)) {
            if (Install-Rust) { $fixesApplied++ }
        }

        # Install WASM target
        if (Install-WasmTarget) { $fixesApplied++ }

        # Clean build artifacts
        if (Clean-BuildArtifacts -Path $ProjectPath) { $fixesApplied++ }

        # Update dependencies
        if (Update-CargoDependencies -Path $ProjectPath) { $fixesApplied++ }

        # Optimize build configuration
        if (Optimize-BuildConfiguration -Path $ProjectPath) { $fixesApplied++ }
    }

    if ($fixesApplied -gt 0) {
        Write-Log "✅ Applied $fixesApplied build fixes" "SUCCESS"
        Write-Host ""
        Write-Log "Testing build after fixes..." "INFO"
        Test-Build -Path $ProjectPath
    }

    # Show final build status
    Write-Host ""
    Write-Log "Final build status:" "INFO"
    Show-BuildStatus -Path $ProjectPath

} catch {
    Write-Log "❌ Script error: $($_.Exception.Message)" "ERROR"
} finally {
    Write-Host ""
    Write-Log "Build issues fix completed" "INFO"
}
