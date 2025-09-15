# AutoAgents GPU Monitor and Optimization Script
# Monitors GPU utilization and provides optimization recommendations

param(
    [int]$MonitorIntervalSeconds = 30,
    [switch]$Optimize
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
        $gpuInfo = nvidia-smi --query-gpu=name,memory.used,memory.total,memory.free,temperature.gpu,utilization.gpu,utilization.memory,power.draw,power.limit --format=csv,noheader,nounits
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
            }
        }
    } catch {
        Write-Log "Failed to get GPU info: $($_.Exception.Message)" "ERROR"
    }
    return $null
}

function Optimize-GPUSettings {
    Write-Log "Applying GPU optimization settings..." "INFO"

    # Set environment variables for maximum performance
    $envVars = @{
        "CUDA_VISIBLE_DEVICES" = "0"
        "TORCH_USE_CUDA_DSA" = "1"
        "CUDA_LAUNCH_BLOCKING" = "0"
        "PYTORCH_CUDA_ALLOC_CONF" = "max_split_size_mb:512"
        "OMP_NUM_THREADS" = "16"
        "MKL_NUM_THREADS" = "16"
    }

    foreach ($var in $envVars.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($var.Key, $var.Value, "Process")
        Write-Log "Set $($var.Key) = $($var.Value)" "SUCCESS"
    }

    # Try to set GPU to maximum performance mode
    try {
        $result = nvidia-smi -pm 1 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Log "Enabled persistent mode for maximum performance" "SUCCESS"
        }
    } catch {
        Write-Log "Could not enable persistent mode" "WARNING"
    }
}

function Monitor-GPU {
    Write-Log "Starting GPU monitoring (interval: ${MonitorIntervalSeconds}s)" "INFO"
    Write-Log "Press Ctrl+C to stop monitoring" "INFO"
    Write-Host ""

    $previousUtilization = 0
    $maxUtilization = 0
    $minUtilization = 100
    $sampleCount = 0

    while ($true) {
        $gpu = Get-GPUInfo

        if ($gpu) {
            $sampleCount++
            $currentUtil = $gpu.GPUUtilization
            $maxUtilization = [Math]::Max($maxUtilization, $currentUtil)
            $minUtilization = [Math]::Min($minUtilization, $currentUtil)

            # Calculate utilization trend
            $trend = if ($currentUtil -gt $previousUtilization) { "↗" }
                    elseif ($currentUtil -lt $previousUtilization) { "↘" }
                    else { "→" }

            Write-Host ("GPU: {0,-25} | Util: {1,3}% {2} | Mem: {3,4}/{4,4} MB ({5,3}%) | Temp: {6,2}°C | Power: {7,4}/{8,4}W" -f
                $gpu.Name,
                $currentUtil,
                $trend,
                $gpu.MemoryUsed,
                $gpu.MemoryTotal,
                $gpu.MemoryUtilization,
                $gpu.Temperature,
                $gpu.PowerDraw,
                $gpu.PowerLimit)

            # Provide optimization recommendations
            if ($currentUtil -lt 50) {
                Write-Log "GPU utilization is low (<50%). Consider increasing batch sizes or model complexity." "WARNING"
            } elseif ($currentUtil -gt 90) {
                Write-Log "GPU utilization is excellent (>90%)!" "SUCCESS"
            }

            if ($gpu.MemoryUtilization -gt 90) {
                Write-Log "GPU memory usage is high (>90%). Consider reducing batch size." "WARNING"
            }

            if ($gpu.Temperature -gt 80) {
                Write-Log "GPU temperature is high (>80°C). Ensure proper cooling." "WARNING"
            }

            $previousUtilization = $currentUtil
        } else {
            Write-Log "No GPU detected or nvidia-smi not available" "ERROR"
            Write-Log "Make sure NVIDIA drivers are installed and nvidia-smi is in PATH" "ERROR"
            break
        }

        Start-Sleep -Seconds $MonitorIntervalSeconds
    }
}

function Show-GPUSummary {
    param([int]$SampleCount, [int]$MaxUtil, [int]$MinUtil)

    Write-Host ""
    Write-Host "GPU Monitoring Summary:" -ForegroundColor Cyan
    Write-Host "======================" -ForegroundColor Cyan
    Write-Host "Samples taken: $SampleCount"
    Write-Host "Max utilization: ${MaxUtil}%"
    Write-Host "Min utilization: ${MinUtil}%"
    Write-Host "Average utilization: $([Math]::Round(($MaxUtil + $MinUtil) / 2, 1))%"

    if ($MaxUtil -gt 80) {
        Write-Log "Excellent GPU utilization achieved!" "SUCCESS"
    } elseif ($MaxUtil -gt 50) {
        Write-Log "Good GPU utilization. Room for improvement." "INFO"
    } else {
        Write-Log "GPU utilization is suboptimal. Check configuration." "WARNING"
    }
}

# Main execution
try {
    Write-Host "AutoAgents GPU Monitor and Optimizer" -ForegroundColor Green
    Write-Host "====================================" -ForegroundColor Green
    Write-Host ""

    if ($Optimize) {
        Optimize-GPUSettings
        Write-Host ""
    }

    # Check if GPU is available
    $gpu = Get-GPUInfo
    if (-not $gpu) {
        Write-Log "No NVIDIA GPU detected. This script requires NVIDIA GPU with drivers." "ERROR"
        exit 1
    }

    Write-Log "GPU detected: $($gpu.Name)" "SUCCESS"
    Write-Log "Memory: $($gpu.MemoryTotal) MB" "INFO"
    Write-Log "Starting monitoring..." "INFO"

    # Start monitoring
    Monitor-GPU

} catch {
    Write-Log "Script error: $($_.Exception.Message)" "ERROR"
} finally {
    Write-Host ""
    Write-Log "GPU monitoring stopped" "INFO"
}
