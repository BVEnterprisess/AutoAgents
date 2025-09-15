# AutoAgents Sync Monitor - Enhanced Sync Monitoring with Detailed Logging
# This script provides comprehensive monitoring of GitHub synchronization operations

param(
    [string]$RepoPath = $PSScriptRoot,
    [int]$MonitorIntervalMinutes = 10,
    [switch]$EnableDetailedLogging,
    [switch]$EnableErrorRecovery,
    [string]$LogFile = "sync_monitor.log"
)

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logEntry = "[$timestamp] [$Level] $Message"

    # Write to console with color
    $color = switch ($Level) {
        "ERROR" { "Red" }
        "WARNING" { "Yellow" }
        "SUCCESS" { "Green" }
        default { "White" }
    }
    Write-Host $logEntry -ForegroundColor $color

    # Write to log file if detailed logging is enabled
    if ($EnableDetailedLogging) {
        try {
            $logEntry | Out-File -FilePath $LogFile -Append -Encoding UTF8
        } catch {
            Write-Host "Failed to write to log file: $($_.Exception.Message)" -ForegroundColor Red
        }
    }
}

function Test-GitConnectivity {
    param([string]$RemoteUrl)

    try {
        Write-Log "Testing Git connectivity to: $RemoteUrl" "INFO"

        # Test basic connectivity
        $pingResult = Test-Connection -ComputerName "github.com" -Count 1 -Quiet
        if (!$pingResult) {
            Write-Log "Network connectivity test failed" "ERROR"
            return $false
        }

        # Test Git remote connectivity
        git ls-remote $RemoteUrl --heads 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Log "Git remote connectivity test passed" "SUCCESS"
            return $true
        } else {
            Write-Log "Git remote connectivity test failed" "ERROR"
            return $false
        }
    } catch {
        Write-Log "Error testing connectivity: $($_.Exception.Message)" "ERROR"
        return $false
    }
}

function Get-RepositoryStatus {
    param([string]$Path)

    try {
        Push-Location $Path

        # Get basic repository information
        $status = git status --porcelain 2>$null
        $branch = git branch --show-current 2>$null
        $remote = git remote get-url origin 2>$null
        $lastCommit = git log -1 --format="%H %s" 2>$null
        $pendingCommits = git rev-list --count HEAD..origin/$branch 2>$null

        $repoInfo = @{
            HasChanges = ($status -and $status.Length -gt 0)
            CurrentBranch = $branch
            RemoteUrl = $remote
            LastCommit = $lastCommit
            PendingCommits = [int]$pendingCommits
            StatusCode = $LASTEXITCODE
        }

        return $repoInfo
    } catch {
        Write-Log "Error getting repository status: $($_.Exception.Message)" "ERROR"
        return $null
    } finally {
        Pop-Location
    }
}

function Test-RepositoryHealth {
    param([string]$Path)

    try {
        Push-Location $Path

        $healthIssues = @()

        # Check for uncommitted changes
        $status = git status --porcelain 2>$null
        if ($status) {
            $healthIssues += "Uncommitted changes detected"
        }

        # Check for untracked files
        $untracked = git ls-files --others --exclude-standard 2>$null
        if ($untracked) {
            $healthIssues += "Untracked files detected"
        }

        # Check for merge conflicts
        $mergeHead = Test-Path ".git/MERGE_HEAD"
        if ($mergeHead) {
            $healthIssues += "Merge in progress"
        }

        # Check repository integrity
        git fsck --no-progress 2>$null | Out-Null
        if ($LASTEXITCODE -ne 0) {
            $healthIssues += "Repository integrity issues detected"
        }

        return @{
            IsHealthy = ($healthIssues.Count -eq 0)
            Issues = $healthIssues
        }
    } catch {
        Write-Log "Error checking repository health: $($_.Exception.Message)" "ERROR"
        return @{
            IsHealthy = $false
            Issues = @("Health check failed: $($_.Exception.Message)")
        }
    } finally {
        Pop-Location
    }
}

function Start-SyncMonitor {
    param([string]$Path, [int]$IntervalMinutes)

    Write-Log "Starting AutoAgents Sync Monitor" "INFO"
    Write-Log "Repository: $Path" "INFO"
    Write-Log "Monitor Interval: $IntervalMinutes minutes" "INFO"
    Write-Log "Detailed Logging: $EnableDetailedLogging" "INFO"
    Write-Log "Error Recovery: $EnableErrorRecovery" "INFO"
    Write-Host ""

    # Validate repository
    if (!(Test-Path $Path)) {
        Write-Log "Repository path does not exist: $Path" "ERROR"
        return
    }

    if (!(Test-Path (Join-Path $Path ".git"))) {
        Write-Log "Not a Git repository: $Path" "ERROR"
        return
    }

    # Get initial repository status
    $initialStatus = Get-RepositoryStatus -Path $Path
    if ($initialStatus) {
        Write-Log "Initial repository status:" "INFO"
        Write-Log "  Branch: $($initialStatus.CurrentBranch)" "INFO"
        Write-Log "  Remote: $($initialStatus.RemoteUrl)" "INFO"
        Write-Log "  Has Changes: $($initialStatus.HasChanges)" "INFO"
        Write-Log "  Pending Commits: $($initialStatus.PendingCommits)" "INFO"
    }

    # Test connectivity
    if ($initialStatus -and $initialStatus.RemoteUrl) {
        $connectivity = Test-GitConnectivity -RemoteUrl $initialStatus.RemoteUrl
        if (!$connectivity -and $EnableErrorRecovery) {
            Write-Log "Connectivity issues detected, will retry during monitoring" "WARNING"
        }
    }

    $cycleCount = 0
    $syncSuccessCount = 0
    $syncFailureCount = 0
    $lastSyncTime = Get-Date

    try {
        while ($true) {
            $cycleCount++
            Write-Log "=== Monitoring Cycle #$cycleCount ===" "INFO"

            # Check repository health
            $health = Test-RepositoryHealth -Path $Path
            if (!$health.IsHealthy) {
                Write-Log "Repository health issues detected:" "WARNING"
                foreach ($issue in $health.Issues) {
                    Write-Log "  - $issue" "WARNING"
                }

                if ($EnableErrorRecovery) {
                    Write-Log "Attempting to resolve health issues..." "INFO"
                    # Attempt basic recovery
                    Push-Location $Path
                    try {
                        # Reset any merge conflicts
                        if (Test-Path ".git/MERGE_HEAD") {
                            git merge --abort 2>$null | Out-Null
                            Write-Log "Aborted ongoing merge" "SUCCESS"
                        }

                        # Clean untracked files if configured
                        # git clean -fd 2>$null | Out-Null  # Uncomment if desired
                    } catch {
                        Write-Log "Error during recovery: $($_.Exception.Message)" "ERROR"
                    } finally {
                        Pop-Location
                    }
                }
            }

            # Get current status
            $currentStatus = Get-RepositoryStatus -Path $Path
            if ($currentStatus) {
                Write-Log "Current Status:" "INFO"
                Write-Log "  Branch: $($currentStatus.CurrentBranch)" "INFO"
                Write-Log "  Has Changes: $($currentStatus.HasChanges)" "INFO"
                Write-Log "  Pending Commits: $($currentStatus.PendingCommits)" "INFO"

                # Check if sync is needed
                $needsSync = $currentStatus.HasChanges -or ($currentStatus.PendingCommits -gt 0)

                if ($needsSync) {
                    Write-Log "Sync needed - changes detected" "INFO"

                    # Attempt sync
                    $syncResult = Sync-Repository -Path $Path
                    if ($syncResult) {
                        $syncSuccessCount++
                        $lastSyncTime = Get-Date
                        Write-Log "Sync completed successfully" "SUCCESS"
                    } else {
                        $syncFailureCount++
                        Write-Log "Sync failed" "ERROR"
                    }
                } else {
                    Write-Log "No sync needed - repository is up to date" "INFO"
                }
            }

            # Show statistics
            $uptime = (Get-Date) - $lastSyncTime
            Write-Log "Statistics:" "INFO"
            Write-Log "  Total Cycles: $cycleCount" "INFO"
            Write-Log "  Successful Syncs: $syncSuccessCount" "INFO"
            Write-Log "  Failed Syncs: $syncFailureCount" "INFO"
            Write-Log "  Time since last sync: $($uptime.TotalMinutes.ToString('F1')) minutes" "INFO"

            # Wait for next cycle
            Write-Log "Waiting $IntervalMinutes minutes until next check..." "INFO"
            Start-Sleep -Seconds ($IntervalMinutes * 60)
        }
    } catch {
        Write-Log "Monitoring stopped due to error: $($_.Exception.Message)" "ERROR"
    } finally {
        Write-Log "Sync monitoring completed" "INFO"
        Write-Log "Final Statistics:" "INFO"
        Write-Log "  Total Cycles: $cycleCount" "INFO"
        Write-Log "  Successful Syncs: $syncSuccessCount" "INFO"
        Write-Log "  Failed Syncs: $syncFailureCount" "INFO"
        Write-Log "  Success Rate: $(if ($cycleCount -gt 0) { [math]::Round(($syncSuccessCount / $cycleCount) * 100, 1) } else { 0 })%" "INFO"
    }
}

function Sync-Repository {
    param([string]$Path)

    try {
        Push-Location $Path

        Write-Log "Starting repository sync..." "INFO"

        # Get current branch
        $currentBranch = git branch --show-current 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "Cannot determine current branch" "ERROR"
            return $false
        }

        # Fetch latest changes
        Write-Log "Fetching latest changes..." "INFO"
        git fetch --all --prune 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "Fetch failed" "ERROR"
            return $false
        }

        # Check for local changes
        $status = git status --porcelain 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "Cannot check status" "ERROR"
            return $false
        }

        if ($status) {
            Write-Log "Committing local changes..." "INFO"
            git add . 2>$null
            if ($LASTEXITCODE -ne 0) {
                Write-Log "Git add failed" "ERROR"
                return $false
            }

            git commit -m "Auto-sync: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" 2>$null
            if ($LASTEXITCODE -ne 0) {
                Write-Log "Git commit failed" "ERROR"
                return $false
            }
        }

        # Pull with rebase
        Write-Log "Pulling latest changes..." "INFO"
        git pull --rebase origin $currentBranch 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "Pull failed, aborting rebase..." "ERROR"
            git rebase --abort 2>$null | Out-Null
            return $false
        }

        # Push changes
        Write-Log "Pushing changes..." "INFO"
        git push origin $currentBranch 2>$null
        if ($LASTEXITCODE -ne 0) {
            Write-Log "Push failed" "ERROR"
            return $false
        }

        Write-Log "Repository sync completed successfully" "SUCCESS"
        return $true

    } catch {
        Write-Log "Error during sync: $($_.Exception.Message)" "ERROR"
        return $false
    } finally {
        Pop-Location
    }
}

# Main execution
try {
    Write-Host "🔄 AutoAgents Sync Monitor" -ForegroundColor Cyan
    Write-Host "=" * 35 -ForegroundColor Cyan
    Write-Host ""

    # Clear log file if it exists and detailed logging is enabled
    if ($EnableDetailedLogging -and (Test-Path $LogFile)) {
        try {
            Clear-Content $LogFile
            Write-Log "Log file cleared: $LogFile" "INFO"
        } catch {
            Write-Log "Could not clear log file: $($_.Exception.Message)" "WARNING"
        }
    }

    # Start monitoring
    Start-SyncMonitor -Path $RepoPath -IntervalMinutes $MonitorIntervalMinutes

} catch {
    Write-Log "❌ Script error: $($_.Exception.Message)" "ERROR"
} finally {
    Write-Host ""
    Write-Log "Sync monitor finished" "INFO"
}
