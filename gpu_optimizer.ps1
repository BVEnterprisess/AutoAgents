# AutoAgents GPU Optimizer - Advanced GPU Memory Management & Optimization
# This script provides comprehensive GPU optimization with CUDA memory tuning

param(
    [switch]$EnablePersistentMode,
    [switch]$OptimizeMemory,
    [switch]$SetMaxClocks,
    [switch]$MonitorContinuously,
    [int]$MonitorIntervalSeconds = 5
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

function Get-GPUInfo {
    try {
        $gpuInfo = nvidia-smi --query-gpu=name,memory.used,memory.total,memory.free,temperature.gpu,utilization.gpu,utilization.memory,power.draw,power.limit,clocks.current.graphics,clocks.max.graphics --format=csv,noheader,nounits
        if ($LASTEXITCODE -eq 0) {
            $parts = $gpuInfo -split ','
            return @{
                Name = $parts[0].Trim()
                MemoryUsed = [int]$parts[1].Trim()
                MemoryTotal = [int]$parts[2].Trim()
                MemoryFree = [int]$parts[3].Trim()
                Temperature = [int]$parts[4].Trim()
                GPUUtilization = [int]$parts[5].Trim()
                MemoryUtilization = [int]$parts[6].Trim()
                PowerDraw = [double]$parts[7].Trim()
                PowerLimit = [double]$parts[8].Trim()
                CurrentClock = [int]$parts[9].Trim()
                MaxClock = [int]$parts[10].Trim()
            }
        }
    } catch {
        Write-Log "Failed to get GPU info: $($_.Exception.Message)" "ERROR"
    }
    return $null
}

function Enable-PersistentMode {
    Write-Log "Enabling GPU persistent mode for maximum performance..." "INFO"

    try {
        $result = nvidia-smi -pm 1 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Log "✅ GPU persistent mode enabled successfully" "SUCCESS"
            return $true
        } else {
            Write-Log "❌ Failed to enable persistent mode" "ERROR"
            return $false
        }
    } catch {
        Write-Log "Error enabling persistent mode: $($_.Exception.Message)" "ERROR"
        return $false
    }
}

function Optimize-GPUMemory {
    Write-Log "Optimizing GPU memory settings..." "INFO"

    # Set CUDA memory management environment variables
    $envVars = @{
        "CUDA_VISIBLE_DEVICES" = "0"
        "CUDA_LAUNCH_BLOCKING" = "0"
        "PYTORCH_CUDA_ALLOC_CONF" = "max_split_size_mb:512,garbage_collection_threshold:0.8"
        "TF_FORCE_GPU_ALLOW_GROWTH" = "true"
        "TF_CPP_MIN_LOG_LEVEL" = "2"
    }

    foreach ($var in $envVars.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($var.Key, $var.Value, "Process")
        Write-Log "Set $($var.Key) = $($var.Value)" "SUCCESS"
    }

    # Clear GPU memory cache if possible
    try {
        # Try to reset GPU memory (this may not work on all systems)
        nvidia-smi --gpu-reset 2>$null | Out-Null
        Write-Log "GPU memory cache cleared" "SUCCESS"
    } catch {
        Write-Log "Could not clear GPU memory cache (normal for some systems)" "WARNING"
    }

    return $true
}

function Set-MaxGPUClocks {
    Write-Log "Setting GPU to maximum performance clocks..." "INFO"

    try {
        # Get current GPU info to determine optimal clocks
        $gpu = Get-GPUInfo
        if (!$gpu) {
            Write-Log "Cannot set clocks - unable to get GPU info" "ERROR"
            return $false
        }

        # Set application clocks to maximum
        nvidia-smi -ac 5001,1590 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Log "✅ GPU application clocks set to maximum" "SUCCESS"
        } else {
            Write-Log "⚠️ Could not set application clocks (may already be optimized)" "WARNING"
        }

        # Lock GPU clock to maximum
        nvidia-smi -lgc 5001 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) {
            Write-Log "✅ GPU clock locked to maximum frequency" "SUCCESS"
        } else {
            Write-Log "⚠️ Could not lock GPU clock (may already be locked)" "WARNING"
        }

        return $true
    } catch {
        Write-Log "Error setting GPU clocks: $($_.Exception.Message)" "ERROR"
        return $false
    }
}

function Monitor-GPUPerformance {
    param([int]$IntervalSeconds = 5)

    Write-Log "Starting continuous GPU performance monitoring..." "INFO"
    Write-Log "Press Ctrl+C to stop monitoring" "WARNING"

    $startTime = Get-Date
    $sampleCount = 0

    try {
        while ($true) {
            $gpu = Get-GPUInfo
            if ($gpu) {
                $sampleCount++
                $elapsed = (Get-Date) - $startTime
                $avgUtilization = [math]::Round($gpu.GPUUtilization, 1)
                $avgMemoryUtil = [math]::Round($gpu.MemoryUtilization, 1)
                $temp = $gpu.Temperature
                $powerPercent = [math]::Round(($gpu.PowerDraw / $gpu.PowerLimit) * 100, 1)

                Write-Host "[$($elapsed.ToString('mm\:ss'))] GPU: ${avgUtilization}% | Mem: $($gpu.MemoryUsed)/$($gpu.MemoryTotal) MB ($avgMemoryUtil%) | Temp: ${temp}°C | Power: ${powerPercent}%" -ForegroundColor Green

                # Performance recommendations
                if ($gpu.GPUUtilization -lt 50) {
                    Write-Host "💡 Tip: GPU utilization is low. Consider increasing workload or optimizing CUDA kernels." -ForegroundColor Yellow
                }
                if ($gpu.Temperature -gt 80) {
                    Write-Host "🔥 Warning: GPU temperature is high. Consider improving cooling." -ForegroundColor Red
                }
                if ($gpu.MemoryUtilization -gt 90) {
                    Write-Host "🧠 Warning: GPU memory usage is very high. Consider optimizing memory allocation." -ForegroundColor Red
                }
            } else {
                Write-Host "❌ Unable to retrieve GPU information" -ForegroundColor Red
            }

            Start-Sleep -Seconds $IntervalSeconds
        }
    } catch {
        Write-Log "Monitoring stopped: $($_.Exception.Message)" "INFO"
    } finally {
        Write-Log "GPU monitoring completed. Total samples: $sampleCount" "SUCCESS"
    }
}

function Show-GPUStatus {
    Write-Log "Current GPU Status:" "INFO"
    Write-Host "=" * 50 -ForegroundColor Cyan

    $gpu = Get-GPUInfo
    if ($gpu) {
        Write-Host "GPU Model: $($gpu.Name)" -ForegroundColor White
        Write-Host "Memory: $($gpu.MemoryUsed) MB / $($gpu.MemoryTotal) MB ($($gpu.MemoryUtilization)%)" -ForegroundColor White
        Write-Host "GPU Utilization: $($gpu.GPUUtilization)%" -ForegroundColor White
        Write-Host "Temperature: $($gpu.Temperature)°C" -ForegroundColor White
        Write-Host "Power: $($gpu.PowerDraw)W / $($gpu.PowerLimit)W" -ForegroundColor White
        Write-Host "Clock Speed: $($gpu.CurrentClock) MHz / $($gpu.MaxClock) MHz" -ForegroundColor White

        # Performance assessment
        $performance = if ($gpu.GPUUtilization -ge 90) { "Excellent" }
                      elseif ($gpu.GPUUtilization -ge 70) { "Good" }
                      elseif ($gpu.GPUUtilization -ge 50) { "Fair" }
                      else { "Poor" }

        Write-Host "Performance Rating: $performance" -ForegroundColor $(if ($performance -eq "Excellent") { "Green" } elseif ($performance -eq "Good") { "Yellow" } else { "Red" })
    } else {
        Write-Host "❌ Unable to retrieve GPU information" -ForegroundColor Red
    }

    Write-Host "=" * 50 -ForegroundColor Cyan
}

# Main execution
try {
    Write-Host "🚀 AutoAgents GPU Optimizer" -ForegroundColor Green
    Write-Host "=" * 40 -ForegroundColor Green
    Write-Host ""

    # Check if NVIDIA GPU is available
    $gpu = Get-GPUInfo
    if (!$gpu) {
        Write-Log "❌ No NVIDIA GPU detected! This tool requires an NVIDIA GPU." "ERROR"
        exit 1
    }

    Write-Log "✅ NVIDIA GPU detected: $($gpu.Name)" "SUCCESS"

    # Show initial status
    Show-GPUStatus

    # Apply optimizations based on parameters
    $optimizationsApplied = 0

    if ($EnablePersistentMode) {
        if (Enable-PersistentMode) { $optimizationsApplied++ }
    }

    if ($OptimizeMemory) {
        if (Optimize-GPUMemory) { $optimizationsApplied++ }
    }

    if ($SetMaxClocks) {
        if (Set-MaxGPUClocks) { $optimizationsApplied++ }
    }

    # If no specific optimizations requested, apply all
    if (!$EnablePersistentMode -and !$OptimizeMemory -and !$SetMaxClocks) {
        Write-Log "No specific optimizations requested, applying all optimizations..." "INFO"
        Enable-PersistentMode | Out-Null
        Optimize-GPUMemory | Out-Null
        Set-MaxGPUClocks | Out-Null
        $optimizationsApplied = 3
    }

    if ($optimizationsApplied -gt 0) {
        Write-Log "✅ Applied $optimizationsApplied GPU optimizations" "SUCCESS"
        Write-Host ""
        Write-Log "Showing optimized GPU status:" "INFO"
        Show-GPUStatus
    }

    # Start monitoring if requested
    if ($MonitorContinuously) {
        Write-Host ""
        Monitor-GPUPerformance -IntervalSeconds $MonitorIntervalSeconds
    }

} catch {
    Write-Log "❌ Script error: $($_.Exception.Message)" "ERROR"
} finally {
    Write-Host ""
    Write-Log "GPU optimization completed" "SUCCESS"
}
