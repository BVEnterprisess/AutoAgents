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

function Sync-Repository {
    param([string]$Path)

    try {
        Write-Log "Starting Git sync for: $Path"

        # Change to repository directory
        Push-Location $Path

        # Check if it's a git repository
        if (!(Test-Path ".git")) {
            Write-Log "ERROR: Not a git repository: $Path"
            return
        }

        # Get current branch
        $currentBranch = git branch --show-current
        Write-Log "Current branch: $currentBranch"

        # Fetch latest changes
        Write-Log "Fetching latest changes..."
        git fetch --all --prune

        # Check for local changes
        $status = git status --porcelain
        if ($status) {
            Write-Log "Local changes detected, staging and committing..."
            git add .
            git commit -m "Auto-sync: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
        }

        # Pull with rebase to avoid merge commits
        Write-Log "Pulling latest changes with rebase..."
        git pull --rebase origin $currentBranch

        # Push local changes
        Write-Log "Pushing changes to remote..."
        git push origin $currentBranch

        # Check GPU utilization if available
        try {
            $gpuInfo = Get-WmiObject -Query "SELECT * FROM Win32_VideoController" | Select-Object Name, DriverVersion, AdapterRAM
            Write-Log "GPU Info: $($gpuInfo.Name) - RAM: $([math]::Round($gpuInfo.AdapterRAM/1GB, 2))GB"
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
