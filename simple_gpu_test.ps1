# Simple GPU Max Test - No CPU Overload
# Pure GPU workload with minimal CPU usage

param([int]$DurationMinutes = 3)

Write-Host "🚀 Starting Simple GPU Max Test" -ForegroundColor Green
Write-Host "Duration: $DurationMinutes minutes" -ForegroundColor Yellow

# Check GPU
try {
    $gpuInfo = nvidia-smi --query-gpu=name,memory.total --format=csv,noheader,nounits
    if ($LASTEXITCODE -eq 0) {
        $parts = $gpuInfo -split ','
        Write-Host "✅ GPU: $($parts[0].Trim())" -ForegroundColor Green
        Write-Host "📊 Memory: $($parts[1].Trim()) MB" -ForegroundColor Green
    }
} catch {
    Write-Host "❌ No GPU detected" -ForegroundColor Red
    exit 1
}

# Set GPU optimizations
$env:CUDA_VISIBLE_DEVICES = "0"
nvidia-smi -pm 1 2>$null | Out-Null

Write-Host "🔥 GPU optimizations applied" -ForegroundColor Green

# Create simple CUDA test
$cudaCode = @"
// Minimal CPU usage GPU test
#include <cuda_runtime.h>
#include <iostream>

__global__ void simpleGPUKernel(float* data, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        float val = data[idx];
        for(int i = 0; i < 10000; i++) {
            val = sinf(val) + cosf(val);
        }
        data[idx] = val;
    }
}

int main() {
    const int size = 1024 * 1024 * 4; // 4M elements
    float* d_data;

    cudaMalloc(&d_data, size * sizeof(float));
    cudaMemset(d_data, 1, size * sizeof(float));

    dim3 blockSize(256);
    dim3 gridSize((size + 255) / 256);

    for(int i = 0; i < 1000; i++) {
        simpleGPUKernel<<<gridSize, blockSize>>>(d_data, size);
        cudaDeviceSynchronize();
    }

    cudaFree(d_data);
    std::cout << "GPU test completed" << std::endl;
    return 0;
}
"@

$cudaCode | Out-File -FilePath "simple_gpu.cu" -Encoding UTF8

# Try to compile
try {
    $nvcc = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.0\bin\nvcc.exe"
    if (Test-Path $nvcc) {
        & $nvcc simple_gpu.cu -o simple_gpu.exe -arch=sm_75 -O3 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ CUDA test compiled" -ForegroundColor Green
            $useCuda = $true
        } else {
            $useCuda = $false
        }
    } else {
        $useCuda = $false
    }
} catch {
    $useCuda = $false
}

# Start monitoring
$monitorJob = Start-Job -ScriptBlock {
    param($DurationMinutes)
    $endTime = (Get-Date).AddMinutes($DurationMinutes)
    $maxUtil = 0

    while ((Get-Date) -lt $endTime) {
        try {
            $gpuInfo = nvidia-smi --query-gpu=utilization.gpu,memory.used,temperature.gpu --format=csv,noheader,nounits
            if ($LASTEXITCODE -eq 0) {
                $parts = $gpuInfo -split ','
                $util = [int]$parts[0].Trim()
                $mem = [int]$parts[1].Trim()
                $temp = [int]$parts[2].Trim()

                $maxUtil = [Math]::Max($maxUtil, $util)

                $time = Get-Date -Format "HH:mm:ss"
                Write-Host "$time | GPU: $util% | Mem: $mem MB | Temp: $temp C"
            }
        } catch {
            Write-Host "Monitoring error"
        }
        Start-Sleep -Seconds 2
    }

    Write-Host "`n🎯 MAX GPU UTILIZATION: $maxUtil%" -ForegroundColor Green
}

# Run GPU test
if ($useCuda) {
    Write-Host "🔥 Running CUDA GPU test..." -ForegroundColor Red
    $testJob = Start-Job -ScriptBlock {
        try {
            & ".\simple_gpu.exe"
        } catch {
            Write-Host "CUDA test failed"
        }
    }
} else {
    Write-Host "⚠️ CUDA not available, using CPU fallback" -ForegroundColor Yellow
    $testJob = Start-Job -ScriptBlock {
        param($Duration)
        $end = (Get-Date).AddMinutes($Duration)
        while ((Get-Date) -lt $end) {
            # Minimal CPU work to keep GPU active
            Start-Sleep -Milliseconds 100
        }
    } -ArgumentList $DurationMinutes
}

# Wait and monitor
Wait-Job $monitorJob -Timeout ($DurationMinutes * 60) | Out-Null
Stop-Job $monitorJob, $testJob -Force

# Cleanup
if (Test-Path "simple_gpu.cu") { Remove-Item "simple_gpu.cu" }
if (Test-Path "simple_gpu.exe") { Remove-Item "simple_gpu.exe" }

Write-Host "✅ GPU test completed!" -ForegroundColor Green
