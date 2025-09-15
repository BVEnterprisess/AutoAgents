# AutoAgents Autonomous Git Sync Script
# Runs every 10 minutes to sync/merge/pull/push codebase to GitHub

param(
    [string]$RepoPath = $PSScriptRoot,
    [int]$IntervalMinutes = 10
)

function Write-Log {
    param([string]$Message)
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    Write-Host "[$timestamp] $Message"
}

function Test-GitRepository {
    param([string]$Path)

    try {
        Push-Location $Path

        # Check if directory exists
        if (!(Test-Path $Path)) {
            Write-Log "ERROR: Directory does not exist: $Path"
            return $false
        }

        # Check if it's a git repository
        if (!(Test-Path ".git")) {
            Write-Log "ERROR: Not a git repository: $Path"
            return $false
        }

        # Validate git installation
        $gitVersion = git --version 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "ERROR: Git is not installed or not in PATH"
            return $false
        }

        # Check repository status
        $status = git status --porcelain 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "ERROR: Cannot access git repository status"
            return $false
        }

        # Check if remote exists
        $remote = git remote 2>$null
        if ($LASTEXITCODE -ne 0 -or !$remote) {
            Write-Log "ERROR: No git remote configured"
            return $false
        }

        Write-Log "Git repository validation passed"
        return $true

    } catch {
        Write-Log "ERROR during repository validation: $($_.Exception.Message)"
        return $false
    } finally {
        Pop-Location
    }
}

function Sync-Repository {
    param([string]$Path)

    try {
        Write-Log "Starting Git sync for: $Path"

        # Validate repository first
        if (!(Test-GitRepository -Path $Path)) {
            Write-Log "Repository validation failed, skipping sync"
            return
        }

        # Change to repository directory
        Push-Location $Path

        # Get current branch with error handling
        try {
            $currentBranch = git branch --show-current 2>$null
            if ($LASTEXITCODE -ne 0) {
                Write-Log "ERROR: Cannot determine current branch"
                return
            }
            Write-Log "Current branch: $currentBranch"
        } catch {
            Write-Log "ERROR: Failed to get current branch: $($_.Exception.Message)"
            return
        }

        # Fetch latest changes with timeout
        Write-Log "Fetching latest changes..."
        $fetchJob = Start-Job -ScriptBlock {
            git fetch --all --prune 2>$null
            return $LASTEXITCODE
        }

        Wait-Job $fetchJob -Timeout 30 | Out-Null
        $fetchResult = Receive-Job $fetchJob
        Remove-Job $fetchJob -Force

        if ($fetchResult -ne 0) {
            Write-Log "ERROR: Git fetch failed or timed out"
            return
        }

        # Check for local changes
        $status = git status --porcelain 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "ERROR: Cannot check git status"
            return
        }

        if ($status) {
            Write-Log "Local changes detected, staging and committing..."
            git add . 2>$null
            if ($LASTEXITCODE -ne 0) {
                Write-Log "ERROR: Git add failed"
                return
            }

            git commit -m "Auto-sync: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" 2>$null
            if ($LASTEXITCODE -ne 0) {
                Write-Log "ERROR: Git commit failed"
                return
            }
        }

        # Pull with rebase to avoid merge commits
        Write-Log "Pulling latest changes with rebase..."
        git pull --rebase origin $currentBranch 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "ERROR: Git pull failed, attempting to abort rebase..."
            git rebase --abort 2>$null | Out-Null
            return
        }

        # Push local changes
        Write-Log "Pushing changes to remote..."
        git push origin $currentBranch 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "ERROR: Git push failed"
            return
        }

        # Check GPU utilization if available
        try {
            $gpuInfo = Get-WmiObject -Query "SELECT * FROM Win32_VideoController" | Select-Object Name, DriverVersion, AdapterRAM
            if ($gpuInfo) {
                Write-Log "GPU Info: $($gpuInfo.Name) - RAM: $([math]::Round($gpuInfo.AdapterRAM/1GB, 2))GB"
            }
        } catch {
            Write-Log "Could not retrieve GPU information"
        }

        Write-Log "Git sync completed successfully"

    } catch {
        Write-Log "ERROR during sync: $($_.Exception.Message)"
    } finally {
        Pop-Location
    }
}

function Start-AutoSync {
    Write-Log "Starting AutoAgents autonomous sync service"
    Write-Log "Repository: $RepoPath"
    Write-Log "Interval: $IntervalMinutes minutes"

    # Initial sync
    Sync-Repository -Path $RepoPath

    # Set up scheduled sync
    while ($true) {
        $nextRun = (Get-Date).AddMinutes($IntervalMinutes)
        Write-Log "Next sync scheduled for: $($nextRun.ToString('yyyy-MM-dd HH:mm:ss'))"

        # Wait for next interval
        Start-Sleep -Seconds ($IntervalMinutes * 60)

        # Perform sync
        Sync-Repository -Path $RepoPath
    }
}

# Check if CUDA is available
try {
    $cudaDevices = & "nvidia-smi" --list-gpus 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Log "CUDA GPU detected - enabling GPU acceleration"
        $env:CUDA_VISIBLE_DEVICES = "0"
    } else {
        Write-Log "No CUDA GPU detected - using CPU mode"
    }
} catch {
    Write-Log "CUDA check failed - using CPU mode"
}

# Start the auto-sync service
Start-AutoSync
