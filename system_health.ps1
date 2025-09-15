# AutoAgents System Health Monitor - Comprehensive System Health Monitoring
# This script provides real-time monitoring of system health with predictive alerts

param(
    [int]$MonitorIntervalSeconds = 30,
    [switch]$EnableAlerts,
    [switch]$EnablePredictiveAnalysis,
    [string]$AlertThresholdsFile = "health_thresholds.json",
    [switch]$ContinuousMonitoring
)

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $color = switch ($Level) {
        "ERROR" { "Red" }
        "WARNING" { "Yellow" }
        "SUCCESS" { "Green" }
        "ALERT" { "Magenta" }
        default { "White" }
    }
    Write-Host "[$timestamp] [$Level] $Message" -ForegroundColor $color
}

function Get-SystemInfo {
    try {
        $cpu = Get-WmiObject -Class Win32_Processor | Select-Object -First 1
        $memory = Get-WmiObject -Class Win32_OperatingSystem
        $disk = Get-WmiObject -Class Win32_LogicalDisk -Filter "DeviceID='C:'"

        return @{
            CPUUsage = (Get-WmiObject -Class Win32_PerfFormattedData_PerfOS_Processor | Where-Object { $_.Name -eq "_Total" }).PercentProcessorTime
            TotalMemory = [math]::Round($memory.TotalVisibleMemorySize / 1MB, 2)
            FreeMemory = [math]::Round($memory.FreePhysicalMemory / 1MB, 2)
            UsedMemory = [math]::Round(($memory.TotalVisibleMemorySize - $memory.FreePhysicalMemory) / 1MB, 2)
            MemoryUsagePercent = [math]::Round((($memory.TotalVisibleMemorySize - $memory.FreePhysicalMemory) / $memory.TotalVisibleMemorySize) * 100, 1)
            TotalDisk = [math]::Round($disk.Size / 1GB, 2)
            FreeDisk = [math]::Round($disk.FreeSpace / 1GB, 2)
            DiskUsagePercent = [math]::Round((($disk.Size - $disk.FreeSpace) / $disk.Size) * 100, 1)
            CPUName = $cpu.Name
            CPUCores = $cpu.NumberOfCores
            CPULogicalProcessors = $cpu.NumberOfLogicalProcessors
        }
    } catch {
        Write-Log "Error getting system info: $($_.Exception.Message)" "ERROR"
        return $null
    }
}

function Get-GPUInfo {
    try {
        $gpuInfo = nvidia-smi --query-gpu=name,memory.used,memory.total,temperature.gpu,utilization.gpu,power.draw,power.limit --format=csv,noheader,nounits 2>$null
        if ($LASTEXITCODE -eq 0) {
            $parts = $gpuInfo -split ','
            return @{
                Name = $parts[0].Trim()
                MemoryUsed = [int]$parts[1].Trim()
                MemoryTotal = [int]$parts[2].Trim()
                Temperature = [int]$parts[4].Trim()
                GPUUtilization = [int]$parts[5].Trim()
                PowerDraw = [double]$parts[6].Trim()
                PowerLimit = [double]$parts[7].Trim()
                MemoryUsagePercent = [math]::Round(($parts[1].Trim() / $parts[2].Trim()) * 100, 1)
                PowerUsagePercent = [math]::Round(($parts[6].Trim() / $parts[7].Trim()) * 100, 1)
            }
        }
    } catch {
        # GPU not available or nvidia-smi not found
    }
    return $null
}

function Get-NetworkInfo {
    try {
        $network = Get-WmiObject -Class Win32_NetworkAdapterConfiguration | Where-Object { $_.IPEnabled -eq $true } | Select-Object -First 1
        $ping = Test-Connection -ComputerName "8.8.8.8" -Count 1 -ErrorAction SilentlyContinue

        return @{
            BytesReceived = $network.BytesReceived
            BytesSent = $network.BytesSent
            PingLatency = if ($ping) { $ping.ResponseTime } else { -1 }
            IsConnected = ($ping -ne $null)
        }
    } catch {
        Write-Log "Error getting network info: $($_.Exception.Message)" "ERROR"
        return $null
    }
}

function Get-ProcessInfo {
    try {
        $processes = Get-Process | Where-Object { $_.CPU -gt 0 } | Sort-Object CPU -Descending | Select-Object -First 10
        $topProcesses = $processes | ForEach-Object {
            @{
                Name = $_.ProcessName
                CPU = [math]::Round($_.CPU, 2)
                MemoryMB = [math]::Round($_.WorkingSet64 / 1MB, 2)
                Id = $_.Id
            }
        }

        return @{
            TotalProcesses = (Get-Process).Count
            TopCPUProcesses = $topProcesses
        }
    } catch {
        Write-Log "Error getting process info: $($_.Exception.Message)" "ERROR"
        return $null
    }
}

function Load-AlertThresholds {
    param([string]$FilePath)

    if (Test-Path $FilePath) {
        try {
            $thresholds = Get-Content $FilePath | ConvertFrom-Json
            Write-Log "Loaded alert thresholds from $FilePath" "SUCCESS"
            return $thresholds
        } catch {
            Write-Log "Error loading thresholds file: $($_.Exception.Message)" "ERROR"
        }
    }

    # Default thresholds
    Write-Log "Using default alert thresholds" "INFO"
    return @{
        CPUUsageWarning = 70
        CPUUsageCritical = 90
        MemoryUsageWarning = 80
        MemoryUsageCritical = 95
        DiskUsageWarning = 85
        DiskUsageCritical = 95
        GPUTempWarning = 75
        GPUTempCritical = 85
        GPUUtilizationWarning = 95
        NetworkLatencyWarning = 100
    }
}

function Check-Thresholds {
    param($SystemInfo, $GPUInfo, $Thresholds)

    $alerts = @()

    # CPU alerts
    if ($SystemInfo.CPUUsage -ge $Thresholds.CPUUsageCritical) {
        $alerts += @{
            Type = "CRITICAL"
            Component = "CPU"
            Message = "CPU usage is critically high: $($SystemInfo.CPUUsage)%"
            Value = $SystemInfo.CPUUsage
            Threshold = $Thresholds.CPUUsageCritical
        }
    } elseif ($SystemInfo.CPUUsage -ge $Thresholds.CPUUsageWarning) {
        $alerts += @{
            Type = "WARNING"
            Component = "CPU"
            Message = "CPU usage is high: $($SystemInfo.CPUUsage)%"
            Value = $SystemInfo.CPUUsage
            Threshold = $Thresholds.CPUUsageWarning
        }
    }

    # Memory alerts
    if ($SystemInfo.MemoryUsagePercent -ge $Thresholds.MemoryUsageCritical) {
        $alerts += @{
            Type = "CRITICAL"
            Component = "Memory"
            Message = "Memory usage is critically high: $($SystemInfo.MemoryUsagePercent)%"
            Value = $SystemInfo.MemoryUsagePercent
            Threshold = $Thresholds.MemoryUsageCritical
        }
    } elseif ($SystemInfo.MemoryUsagePercent -ge $Thresholds.MemoryUsageWarning) {
        $alerts += @{
            Type = "WARNING"
            Component = "Memory"
            Message = "Memory usage is high: $($SystemInfo.MemoryUsagePercent)%"
            Value = $SystemInfo.MemoryUsagePercent
            Threshold = $Thresholds.MemoryUsageWarning
        }
    }

    # Disk alerts
    if ($SystemInfo.DiskUsagePercent -ge $Thresholds.DiskUsageCritical) {
        $alerts += @{
            Type = "CRITICAL"
            Component = "Disk"
            Message = "Disk usage is critically high: $($SystemInfo.DiskUsagePercent)%"
            Value = $SystemInfo.DiskUsagePercent
            Threshold = $Thresholds.DiskUsageCritical
        }
    } elseif ($SystemInfo.DiskUsagePercent -ge $Thresholds.DiskUsageWarning) {
        $alerts += @{
            Type = "WARNING"
            Component = "Disk"
            Message = "Disk usage is high: $($SystemInfo.DiskUsagePercent)%"
            Value = $SystemInfo.DiskUsagePercent
            Threshold = $Thresholds.DiskUsageWarning
        }
    }

    # GPU alerts (if available)
    if ($GPUInfo) {
        if ($GPUInfo.Temperature -ge $Thresholds.GPUTempCritical) {
            $alerts += @{
                Type = "CRITICAL"
                Component = "GPU"
                Message = "GPU temperature is critically high: $($GPUInfo.Temperature)°C"
                Value = $GPUInfo.Temperature
                Threshold = $Thresholds.GPUTempCritical
            }
        } elseif ($GPUInfo.Temperature -ge $Thresholds.GPUTempWarning) {
            $alerts += @{
                Type = "WARNING"
                Component = "GPU"
                Message = "GPU temperature is high: $($GPUInfo.Temperature)°C"
                Value = $GPUInfo.Temperature
                Threshold = $Thresholds.GPUTempWarning
            }
        }

        if ($GPUInfo.GPUUtilization -ge $Thresholds.GPUUtilizationWarning) {
            $alerts += @{
                Type = "WARNING"
                Component = "GPU"
                Message = "GPU utilization is very high: $($GPUInfo.GPUUtilization)%"
                Value = $GPUInfo.GPUUtilization
                Threshold = $Thresholds.GPUUtilizationWarning
            }
        }
    }

    return $alerts
}

function Show-SystemStatus {
    param($SystemInfo, $GPUInfo, $NetworkInfo, $ProcessInfo)

    Write-Host "`n📊 SYSTEM HEALTH STATUS" -ForegroundColor Cyan
    Write-Host "=" * 50 -ForegroundColor Cyan

    # System Information
    Write-Host "🖥️  SYSTEM INFO:" -ForegroundColor White
    Write-Host "   CPU: $($SystemInfo.CPUName)" -ForegroundColor White
    Write-Host "   Cores: $($SystemInfo.CPUCores) physical, $($SystemInfo.CPULogicalProcessors) logical" -ForegroundColor White
    Write-Host "   Memory: $($SystemInfo.TotalMemory) GB total" -ForegroundColor White
    Write-Host "   Disk: $($SystemInfo.TotalDisk) GB total" -ForegroundColor White

    # Performance Metrics
    Write-Host "`n⚡ PERFORMANCE METRICS:" -ForegroundColor Yellow
    Write-Host "   CPU Usage: $($SystemInfo.CPUUsage)% | Memory: $($SystemInfo.UsedMemory)GB/$($SystemInfo.TotalMemory)GB ($($SystemInfo.MemoryUsagePercent)%)" -ForegroundColor White
    Write-Host "   Disk Usage: $([math]::Round($SystemInfo.TotalDisk - $SystemInfo.FreeDisk, 2))GB/$($SystemInfo.TotalDisk)GB ($($SystemInfo.DiskUsagePercent)%)" -ForegroundColor White

    # GPU Information (if available)
    if ($GPUInfo) {
        Write-Host "`n🎮 GPU STATUS:" -ForegroundColor Green
        Write-Host "   GPU: $($GPUInfo.Name)" -ForegroundColor White
        Write-Host "   Temperature: $($GPUInfo.Temperature)°C | Utilization: $($GPUInfo.GPUUtilization)%" -ForegroundColor White
        Write-Host "   Memory: $($GPUInfo.MemoryUsed)MB/$($GPUInfo.MemoryTotal)MB ($($GPUInfo.MemoryUsagePercent)%)" -ForegroundColor White
        Write-Host "   Power: $($GPUInfo.PowerDraw)W/$($GPUInfo.PowerLimit)W ($($GPUInfo.PowerUsagePercent)%)" -ForegroundColor White
    }

    # Network Information
    if ($NetworkInfo) {
        Write-Host "`n🌐 NETWORK STATUS:" -ForegroundColor Blue
        $latencyColor = if ($NetworkInfo.PingLatency -gt 100) { "Red" } elseif ($NetworkInfo.PingLatency -gt 50) { "Yellow" } else { "Green" }
        Write-Host "   Connectivity: $(if ($NetworkInfo.IsConnected) { "Connected" } else { "Disconnected" }) | Latency: $($NetworkInfo.PingLatency)ms" -ForegroundColor $latencyColor
    }

    # Top Processes
    if ($ProcessInfo) {
        Write-Host "`n🔥 TOP PROCESSES (CPU):" -ForegroundColor Red
        $ProcessInfo.TopCPUProcesses | ForEach-Object {
            Write-Host "   $($_.Name): $($_.CPU)% CPU, $($_.MemoryMB)MB RAM" -ForegroundColor White
        }
    }
}

function Start-HealthMonitor {
    param([int]$IntervalSeconds, [switch]$EnableAlerts, [switch]$EnablePredictiveAnalysis)

    Write-Log "Starting AutoAgents System Health Monitor" "INFO"
    Write-Log "Monitor Interval: $IntervalSeconds seconds" "INFO"
    Write-Log "Alerts Enabled: $EnableAlerts" "INFO"
    Write-Log "Predictive Analysis: $EnablePredictiveAnalysis" "INFO"

    # Load alert thresholds
    $thresholds = Load-AlertThresholds -FilePath $AlertThresholdsFile

    # Initialize monitoring data
    $monitoringData = @{
        Samples = @()
        StartTime = Get-Date
        AlertCount = 0
        LastAlertTime = $null
    }

    try {
        while ($true) {
            $sampleTime = Get-Date

            # Collect system information
            $systemInfo = Get-SystemInfo
            $gpuInfo = Get-GPUInfo
            $networkInfo = Get-NetworkInfo
            $processInfo = Get-ProcessInfo

            if ($systemInfo) {
                # Store sample for predictive analysis
                $sample = @{
                    Timestamp = $sampleTime
                    SystemInfo = $systemInfo
                    GPUInfo = $gpuInfo
                    NetworkInfo = $networkInfo
                }
                $monitoringData.Samples += $sample

                # Keep only last 100 samples for analysis
                if ($monitoringData.Samples.Count -gt 100) {
                    $monitoringData.Samples = $monitoringData.Samples[-100..-1]
                }

                # Check thresholds and generate alerts
                if ($EnableAlerts) {
                    $alerts = Check-Thresholds -SystemInfo $systemInfo -GPUInfo $gpuInfo -Thresholds $thresholds

                    foreach ($alert in $alerts) {
                        Write-Log "$($alert.Type): $($alert.Message)" "ALERT"
                        $monitoringData.AlertCount++
                        $monitoringData.LastAlertTime = $sampleTime
                    }
                }

                # Show current status
                Show-SystemStatus -SystemInfo $systemInfo -GPUInfo $gpuInfo -NetworkInfo $networkInfo -ProcessInfo $processInfo

                # Predictive analysis
                if ($EnablePredictiveAnalysis -and $monitoringData.Samples.Count -ge 10) {
                    $predictions = Analyze-PredictiveTrends -MonitoringData $monitoringData
                    if ($predictions) {
                        Write-Host "`n🔮 PREDICTIVE ANALYSIS:" -ForegroundColor Magenta
                        foreach ($prediction in $predictions) {
                            Write-Host "   $prediction" -ForegroundColor Magenta
                        }
                    }
                }

                # Show monitoring statistics
                $uptime = $sampleTime - $monitoringData.StartTime
                Write-Host "`n📈 MONITORING STATS:" -ForegroundColor Cyan
                Write-Host "   Uptime: $($uptime.TotalMinutes.ToString('F1')) minutes" -ForegroundColor White
                Write-Host "   Samples Collected: $($monitoringData.Samples.Count)" -ForegroundColor White
                Write-Host "   Alerts Generated: $($monitoringData.AlertCount)" -ForegroundColor White
                if ($monitoringData.LastAlertTime) {
                    $timeSinceLastAlert = $sampleTime - $monitoringData.LastAlertTime
                    Write-Host "   Time since last alert: $($timeSinceLastAlert.TotalMinutes.ToString('F1')) minutes" -ForegroundColor White
                }
            } else {
                Write-Log "Failed to collect system information" "ERROR"
            }

            # Wait for next sample
            Write-Host "`n⏳ Next update in $IntervalSeconds seconds..." -ForegroundColor Gray
            Start-Sleep -Seconds $IntervalSeconds
        }
    } catch {
        Write-Log "Monitoring stopped due to error: $($_.Exception.Message)" "ERROR"
    } finally {
        Write-Log "System health monitoring completed" "INFO"
        Write-Log "Final Statistics:" "INFO"
        Write-Log "  Total Samples: $($monitoringData.Samples.Count)" "INFO"
        Write-Log "  Total Alerts: $($monitoringData.AlertCount)" "INFO"
        Write-Log "  Monitoring Duration: $((Get-Date) - $monitoringData.StartTime)" "INFO"
    }
}

function Analyze-PredictiveTrends {
    param($MonitoringData)

    $predictions = @()

    if ($MonitoringData.Samples.Count -lt 10) {
        return $predictions
    }

    # Analyze CPU usage trend
    $recentSamples = $MonitoringData.Samples[-10..-1]
    $cpuTrend = $recentSamples | ForEach-Object { $_.SystemInfo.CPUUsage } | Where-Object { $_ -ne $null }

    if ($cpuTrend.Count -ge 5) {
        $avgCPU = ($cpuTrend | Measure-Object -Average).Average
        $cpuSlope = ($cpuTrend[-1] - $cpuTrend[0]) / ($cpuTrend.Count - 1)

        if ($cpuSlope -gt 2) {
            $predictions += "CPU usage is trending upward (+$([math]::Round($cpuSlope, 1))% per sample)"
        } elseif ($cpuSlope -lt -2) {
            $predictions += "CPU usage is trending downward ($([math]::Round($cpuSlope, 1))% per sample)"
        }

        if ($avgCPU -gt 80) {
            $predictions += "High sustained CPU usage detected (avg: $([math]::Round($avgCPU, 1))%)"
        }
    }

    # Analyze memory usage trend
    $memoryTrend = $recentSamples | ForEach-Object { $_.SystemInfo.MemoryUsagePercent } | Where-Object { $_ -ne $null }

    if ($memoryTrend.Count -ge 5) {
        $memorySlope = ($memoryTrend[-1] - $memoryTrend[0]) / ($memoryTrend.Count - 1)

        if ($memorySlope -gt 1) {
            $predictions += "Memory usage is trending upward (+$([math]::Round($memorySlope, 1))% per sample)"
        }
    }

    return $predictions
}

# Main execution
try {
    Write-Host "🏥 AutoAgents System Health Monitor" -ForegroundColor Green
    Write-Host "=" * 40 -ForegroundColor Green
    Write-Host ""

    if ($ContinuousMonitoring) {
        Start-HealthMonitor -IntervalSeconds $MonitorIntervalSeconds -EnableAlerts:$EnableAlerts -EnablePredictiveAnalysis:$EnablePredictiveAnalysis
    } else {
        # Single status check
        $systemInfo = Get-SystemInfo
        $gpuInfo = Get-GPUInfo
        $networkInfo = Get-NetworkInfo
        $processInfo = Get-ProcessInfo

        if ($systemInfo) {
            Show-SystemStatus -SystemInfo $systemInfo -GPUInfo $gpuInfo -NetworkInfo $networkInfo -ProcessInfo $processInfo

            if ($EnableAlerts) {
                $thresholds = Load-AlertThresholds -FilePath $AlertThresholdsFile
                $alerts = Check-Thresholds -SystemInfo $systemInfo -GPUInfo $gpuInfo -Thresholds $thresholds

                if ($alerts.Count -gt 0) {
                    Write-Host "`n🚨 ALERTS:" -ForegroundColor Red
                    foreach ($alert in $alerts) {
                        Write-Host "   $($alert.Type): $($alert.Message)" -ForegroundColor Red
                    }
                } else {
                    Write-Host "`n✅ No alerts - system is healthy" -ForegroundColor Green
                }
            }
        } else {
            Write-Log "Failed to retrieve system information" "ERROR"
        }
    }

} catch {
    Write-Log "❌ Script error: $($_.Exception.Message)" "ERROR"
} finally {
    Write-Host ""
    Write-Log "System health check completed" "INFO"
}
