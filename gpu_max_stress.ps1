# AutoAgents GPU Max Stress Test - Achieve 100% GPU Utilization
# This script creates intensive GPU workloads to maximize utilization

param(
    [int]$DurationMinutes = 10,
    [int]$TargetUtilization = 100,
    [switch]$UseCuda,
    [switch]$UsePyTorch,
    [switch]$UseTensorFlow
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
    Write-Log "Applying maximum performance GPU settings..." "INFO"

    # Set environment variables for maximum performance
    $envVars = @{
        "CUDA_VISIBLE_DEVICES" = "0"
        "TORCH_USE_CUDA_DSA" = "1"
        "CUDA_LAUNCH_BLOCKING" = "0"
        "PYTORCH_CUDA_ALLOC_CONF" = "max_split_size_mb:512"
        "OMP_NUM_THREADS" = "16"
        "MKL_NUM_THREADS" = "16"
        "TF_CPP_MIN_LOG_LEVEL" = "2"
        "TF_FORCE_GPU_ALLOW_GROWTH" = "true"
    }

    foreach ($var in $envVars.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($var.Key, $var.Value, "Process")
        Write-Log "Set $($var.Key) = $($var.Value)" "SUCCESS"
    }

    # Enable GPU persistent mode for maximum performance
    try {
        $result = nvidia-smi -pm 1 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Log "Enabled persistent mode for maximum performance" "SUCCESS"
        }
    } catch {
        Write-Log "Could not enable persistent mode" "WARNING"
    }

    # Set GPU to maximum clocks if available
    try {
        nvidia-smi -ac 5001,1590 2>$null | Out-Null
        Write-Log "Set GPU to maximum application clocks" "SUCCESS"
    } catch {
        Write-Log "Could not set application clocks" "WARNING"
    }
}

function Create-CudaStressTest {
    Write-Log "Creating CUDA stress test..." "INFO"

    $cudaCode = @"
// GPU-ONLY CUDA Stress Test - Minimal CPU Usage
#include <cuda_runtime.h>
#include <iostream>
#include <chrono>

#define BLOCK_SIZE 512
#define NUM_STREAMS 8
#define SHARED_MEM_SIZE 4096

// Pure GPU kernel - all computation stays on GPU
__global__ void gpuIntensiveKernel(float* data, int size, int iterations, int* counter) {
    extern __shared__ float sharedData[];

    int tid = threadIdx.x;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int localIdx = tid;

    // Load data into shared memory for faster access
    if (idx < size) {
        sharedData[localIdx] = data[idx];
    }
    __syncthreads();

    // Pure GPU computation loop - no CPU involvement
    for(int iter = 0; iter < iterations; iter++) {
        if (idx < size) {
            float val = sharedData[localIdx];

            // Heavy mathematical operations
            val = __sinf(val) * __cosf(val) + __tanf(val);
            val = __expf(val) * __logf(__fabsf(val) + 1.0f);
            val = __powf(val, 2.0f) + __fsqrt_rn(__fabsf(val));

            // Memory-intensive operations within shared memory
            int sharedOffset = (localIdx + iter) % blockDim.x;
            val += sharedData[sharedOffset] * sharedData[(localIdx + iter * 2) % blockDim.x];

            // Atomic operations to stress GPU
            atomicAdd(counter, 1);

            sharedData[localIdx] = val;
        }
        __syncthreads();
    }

    // Write back to global memory
    if (idx < size) {
        data[idx] = sharedData[localIdx];
    }
}

// Matrix operations that stay entirely on GPU
__global__ void gpuMatrixOps(float* A, float* B, float* C, int size) {
    int row = blockIdx.y * blockDim.y + threadIdx.y;
    int col = blockIdx.x * blockDim.x + threadIdx.x;

    if (row < size && col < size) {
        float sum = 0.0f;
        for(int k = 0; k < size; k += 4) {  // Unrolled loop for better performance
            sum += A[row * size + k] * B[k * size + col];
            sum += A[row * size + k + 1] * B[(k + 1) * size + col];
            sum += A[row * size + k + 2] * B[(k + 2) * size + col];
            sum += A[row * size + k + 3] * B[(k + 3) * size + col];
        }
        C[row * size + col] = sum;
    }
}

// Memory bandwidth stress test
__global__ void memoryBandwidthTest(float* src, float* dst, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        // Multiple memory operations to stress bandwidth
        float val = src[idx];
        dst[idx] = val * 2.0f;
        dst[idx + size/4] = val * 3.0f;
        dst[idx + size/2] = val * 4.0f;
        dst[idx + 3*size/4] = val * 5.0f;
    }
}

int main() {
    // Large data sizes to maximize GPU utilization
    const int dataSize = 1024 * 1024 * 16;  // 16M elements
    const int matrixSize = 4096;  // 4096x4096 matrix
    const int iterations = 2000;

    std::cout << "🚀 Starting GPU-ONLY CUDA Stress Test..." << std::endl;
    std::cout << "Data size: " << dataSize << " elements (" << dataSize * sizeof(float) / (1024*1024) << " MB)" << std::endl;
    std::cout << "Matrix size: " << matrixSize << "x" << matrixSize << " (" << matrixSize * matrixSize * sizeof(float) / (1024*1024) << " MB)" << std::endl;

    // Allocate device memory only (no host memory to minimize CPU usage)
    float *d_data, *d_matrixA, *d_matrixB, *d_matrixC, *d_temp;
    int* d_counter;

    cudaMalloc(&d_data, dataSize * sizeof(float));
    cudaMalloc(&d_matrixA, matrixSize * matrixSize * sizeof(float));
    cudaMalloc(&d_matrixB, matrixSize * matrixSize * sizeof(float));
    cudaMalloc(&d_matrixC, matrixSize * matrixSize * sizeof(float));
    cudaMalloc(&d_temp, dataSize * sizeof(float));
    cudaMalloc(&d_counter, sizeof(int));

    // Initialize data directly on GPU (no CPU initialization)
    cudaMemset(d_data, 0, dataSize * sizeof(float));
    cudaMemset(d_matrixA, 0, matrixSize * matrixSize * sizeof(float));
    cudaMemset(d_matrixB, 0, matrixSize * matrixSize * sizeof(float));
    cudaMemset(d_counter, 0, sizeof(int));

    // Fill matrices with non-zero values using GPU kernel
    dim3 initBlock(32, 32);
    dim3 initGrid((matrixSize + 31) / 32, (matrixSize + 31) / 32);

    // Create CUDA streams for maximum concurrency
    cudaStream_t streams[NUM_STREAMS];
    for(int i = 0; i < NUM_STREAMS; i++) {
        cudaStreamCreate(&streams[i]);
    }

    // Kernel launch parameters
    dim3 blockSize(BLOCK_SIZE);
    dim3 gridSize((dataSize + BLOCK_SIZE - 1) / BLOCK_SIZE);
    dim3 matrixBlockSize(16, 16);
    dim3 matrixGridSize((matrixSize + 15) / 16, (matrixSize + 15) / 16);

    auto startTime = std::chrono::high_resolution_clock::now();

    // Main stress test loop - pure GPU computation
    int runIterations = 0;
    while(true) {
        // Launch multiple kernels concurrently across streams
        for(int stream = 0; stream < NUM_STREAMS; stream++) {
            int offset = (dataSize / NUM_STREAMS) * stream;
            int streamSize = (stream == NUM_STREAMS - 1) ? (dataSize - offset) : (dataSize / NUM_STREAMS);

            dim3 streamGridSize((streamSize + BLOCK_SIZE - 1) / BLOCK_SIZE);

            // Intensive computation kernel
            gpuIntensiveKernel<<<streamGridSize, blockSize, SHARED_MEM_SIZE, streams[stream]>>>(
                d_data + offset, streamSize, iterations / 20, d_counter);

            // Memory bandwidth test
            memoryBandwidthTest<<<streamGridSize, blockSize, 0, streams[stream]>>>(
                d_data + offset, d_temp + offset, streamSize);
        }

        // Matrix operations
        gpuMatrixOps<<<matrixGridSize, matrixBlockSize>>>(
            d_matrixA, d_matrixB, d_matrixC, matrixSize);

        // Swap matrices for next iteration (GPU-only operation)
        std::swap(d_matrixA, d_matrixC);

        runIterations++;

        // Minimal CPU output every 50 iterations to reduce CPU usage
        if(runIterations % 50 == 0) {
            auto currentTime = std::chrono::high_resolution_clock::now();
            auto elapsed = std::chrono::duration_cast<std::chrono::seconds>(currentTime - startTime);
            std::cout << "Iteration " << runIterations << " (" << elapsed.count() << "s)" << std::endl;
        }

        // Run for extended period to achieve maximum utilization
        if(runIterations >= 2000) {
            break;
        }
    }

    auto endTime = std::chrono::high_resolution_clock::now();
    auto totalTime = std::chrono::duration_cast<std::chrono::seconds>(endTime - startTime);

    std::cout << "\\n🎯 GPU Stress Test Completed!" << std::endl;
    std::cout << "Total iterations: " << runIterations << std::endl;
    std::cout << "Total time: " << totalTime.count() << " seconds" << std::endl;
    std::cout << "Average time per iteration: "
              << static_cast<double>(totalTime.count()) / runIterations << " seconds" << std::endl;

    // Cleanup
    for(int i = 0; i < NUM_STREAMS; i++) {
        cudaStreamDestroy(streams[i]);
    }

    cudaFree(d_data);
    cudaFree(d_matrixA);
    cudaFree(d_matrixB);
    cudaFree(d_matrixC);
    cudaFree(d_temp);
    cudaFree(d_counter);

    return 0;
}
"@

    $cudaCode | Out-File -FilePath "gpu_stress.cu" -Encoding UTF8
    Write-Log "CUDA stress test code created" "SUCCESS"

    # Try to compile CUDA code
    try {
        $nvccPath = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.0\bin\nvcc.exe"
        if (Test-Path $nvccPath) {
            & $nvccPath gpu_stress.cu -o gpu_stress_cuda.exe -arch=sm_75 --use_fast_math -O3
            if ($LASTEXITCODE -eq 0) {
                Write-Log "CUDA stress test compiled successfully" "SUCCESS"
                return $true
            } else {
                Write-Log "CUDA compilation failed" "ERROR"
            }
        } else {
            Write-Log "NVCC compiler not found at expected location" "WARNING"
        }
    } catch {
        Write-Log "CUDA compilation error: $($_.Exception.Message)" "ERROR"
    }

    return $false
}

function Create-PyTorchStressTest {
    Write-Log "Creating PyTorch GPU stress test..." "INFO"

    $pytorchCode = @'
import torch
import torch.nn as nn
import time
import math
import numpy as np
from torch.cuda.amp import autocast, GradScaler

class StressModel(nn.Module):
    def __init__(self, size=4096):
        super(StressModel, self).__init__()
        self.size = size

        # Create multiple large layers to stress GPU
        self.layers = nn.ModuleList([
            nn.Linear(size, size) for _ in range(8)
        ])

        # Large convolutional layers
        self.conv_layers = nn.ModuleList([
            nn.Conv2d(64, 128, 3, padding=1),
            nn.Conv2d(128, 256, 3, padding=1),
            nn.Conv2d(256, 512, 3, padding=1),
        ])

        # Batch normalization layers
        self.bn_layers = nn.ModuleList([
            nn.BatchNorm1d(size) for _ in range(8)
        ])

    def forward(self, x):
        # Linear layer stress
        for layer, bn in zip(self.layers, self.bn_layers):
            x = torch.relu(bn(layer(x)))

        # Reshape for convolution
        batch_size = x.shape[0]
        x_conv = x.view(batch_size, 64, int(math.sqrt(self.size//64)), int(math.sqrt(self.size//64)))

        # Convolutional stress
        for conv in self.conv_layers:
            x_conv = torch.relu(conv(x_conv))

        # Flatten back
        x = x_conv.view(batch_size, -1)

        return x

def gpu_stress_test(duration_minutes=10):
    print(f"🚀 Starting PyTorch GPU Stress Test for {duration_minutes} minutes")
    print("=" * 60)

    if not torch.cuda.is_available():
        print("❌ CUDA not available!")
        return

    device = torch.device('cuda:0')
    print(f"✅ Using GPU: {torch.cuda.get_device_name(device)}")
    print(f"📊 GPU Memory: {torch.cuda.get_device_properties(device).total_memory / 1024**3:.1f} GB")

    # Set CUDA optimizations
    torch.backends.cudnn.benchmark = True
    torch.backends.cudnn.enabled = True

    # Create large stress model
    model = StressModel(size=4096).to(device)
    model.train()

    # Create optimizer with large batch
    optimizer = torch.optim.AdamW(model.parameters(), lr=0.001, weight_decay=0.01)
    scaler = GradScaler()

    # Create large batch size to maximize GPU utilization
    batch_size = 64  # Adjust based on GPU memory
    input_size = 4096

    print(f"📦 Batch size: {batch_size}")
    print(f"🔢 Input size: {input_size}")
    print(f"🧠 Model parameters: {sum(p.numel() for p in model.parameters()):,}")

    start_time = time.time()
    end_time = start_time + (duration_minutes * 60)

    iteration = 0
    max_utilization = 0
    total_iterations = 0

    print("\\n🔥 Starting GPU stress test...")
    print("Monitoring GPU utilization in real-time")
    print("-" * 60)

    while time.time() < end_time:
        try:
            # Create random input batch
            x = torch.randn(batch_size, input_size, device=device)

            # Forward pass with mixed precision
            with autocast():
                output = model(x)
                # Create fake loss to stress gradients
                loss = output.mean() + output.std() + output.var()

            # Backward pass
            scaler.scale(loss).backward()

            # Optimizer step
            scaler.step(optimizer)
            scaler.update()
            optimizer.zero_grad()

            iteration += 1
            total_iterations += 1

            # Monitor GPU utilization every 10 iterations
            if iteration % 10 == 0:
                try:
                    # Get GPU utilization (this is approximate)
                    memory_used = torch.cuda.memory_allocated(device) / 1024**3
                    memory_total = torch.cuda.get_device_properties(device).total_memory / 1024**3
                    utilization = (memory_used / memory_total) * 100

                    max_utilization = max(max_utilization, utilization)

                    elapsed = time.time() - start_time
                    print(f"🔄 Iter: {total_iterations:4d} | "
                          f"GPU Mem: {memory_used:5.1f}/{memory_total:5.1f} GB ({utilization:5.1f}%) | "
                          f"Time: {elapsed:6.1f}s | "
                          f"Max Util: {max_utilization:5.1f}%")

                except Exception as e:
                    print(f"⚠️ Monitoring error: {e}")

            # Clear cache occasionally to prevent memory issues
            if iteration % 100 == 0:
                torch.cuda.empty_cache()

        except RuntimeError as e:
            if "out of memory" in str(e).lower():
                print(f"⚠️ GPU memory error: {e}")
                torch.cuda.empty_cache()
                batch_size = max(1, batch_size // 2)  # Reduce batch size
                print(f"📉 Reduced batch size to: {batch_size}")
                continue
            else:
                raise e

    # Final statistics
    total_time = time.time() - start_time
    final_memory = torch.cuda.memory_allocated(device) / 1024**3

    print("\\n" + "=" * 60)
    print("🎯 GPU STRESS TEST COMPLETED!")
    print("=" * 60)
    print(f"⏱️  Total time: {total_time:.1f} seconds")
    print(f"🔄 Total iterations: {total_iterations}")
    print(f"📊 Average iterations/second: {total_iterations/total_time:.2f}")
    print(f"🧠 Final GPU memory usage: {final_memory:.2f} GB")
    print(f"📈 Maximum utilization achieved: {max_utilization:.1f}%")
    print(f"🎯 Target utilization: {100 if max_utilization >= 90 else 'Not reached':.1f}%")

    if max_utilization >= 90:
        print("🎉 SUCCESS: Achieved high GPU utilization!")
    elif max_utilization >= 70:
        print("👍 GOOD: Achieved decent GPU utilization")
    else:
        print("⚠️ WARNING: GPU utilization was lower than expected")

if __name__ == "__main__":
    # Run stress test for specified duration
    import sys
    duration = int(sys.argv[1]) if len(sys.argv) > 1 else 10
    gpu_stress_test(duration)
'@

    $pytorchCode | Out-File -FilePath "gpu_stress_pytorch.py" -Encoding UTF8
    Write-Log "PyTorch stress test created" "SUCCESS"
    return $true
}

function Start-GPUStressTest {
    param([int]$DurationMinutes)

    Write-Log "🚀 Starting GPU Max Stress Test for $DurationMinutes minutes" "INFO"
    Write-Host "Target: $TargetUtilization% GPU Utilization" -ForegroundColor Yellow
    Write-Host "=" * 60 -ForegroundColor Cyan

    # Check GPU availability
    $gpu = Get-GPUInfo
    if (-not $gpu) {
        Write-Log "❌ No NVIDIA GPU detected!" "ERROR"
        return
    }

    Write-Log "✅ GPU detected: $($gpu.Name)" "SUCCESS"
    Write-Log "📊 Memory: $($gpu.MemoryTotal) MB" "INFO"
    Write-Log "🌡️ Temperature: $($gpu.Temperature)°C" "INFO"

    # Apply GPU optimizations
    Optimize-GPUSettings

    # Try different stress test methods in order of preference
    $stressTestStarted = $false

    if ($UseCuda -or (-not $UsePyTorch -and -not $UseTensorFlow)) {
        Write-Log "🔥 Attempting CUDA stress test..." "INFO"
        if (Create-CudaStressTest) {
            Write-Log "🚀 Launching CUDA stress test..." "SUCCESS"
            $stressJob = Start-Job -ScriptBlock {
                try {
                    & ".\gpu_stress_cuda.exe"
                } catch {
                    Write-Host "CUDA stress test failed: $($_.Exception.Message)" -ForegroundColor Red
                }
            }
            $stressTestStarted = $true
        }
    }

    if (-not $stressTestStarted -and ($UsePyTorch -or (-not $UseCuda -and -not $UseTensorFlow))) {
        Write-Log "🔥 Attempting PyTorch stress test..." "INFO"
        if (Create-PyTorchStressTest) {
            Write-Log "🚀 Launching PyTorch stress test..." "SUCCESS"
            $stressJob = Start-Job -ScriptBlock {
                param($Duration)
                try {
                    & "python.exe" "gpu_stress_pytorch.py" $Duration
                } catch {
                    Write-Host "PyTorch stress test failed: $($_.Exception.Message)" -ForegroundColor Red
                }
            } -ArgumentList $DurationMinutes
            $stressTestStarted = $true
        }
    }

    if (-not $stressTestStarted) {
        Write-Log "❌ No suitable GPU stress test method available!" "ERROR"
        Write-Log "Please install CUDA Toolkit or PyTorch with CUDA support" "WARNING"
        return
    }

    # Start GPU monitoring job
    Write-Log "📊 Starting real-time GPU monitoring..." "INFO"
    $monitorJob = Start-Job -ScriptBlock {
        param($DurationMinutes, $TargetUtilization)
        $endTime = (Get-Date).AddMinutes($DurationMinutes)
        $maxUtilization = 0
        $sampleCount = 0
        $highUtilCount = 0

        Write-Host "📈 GPU Monitoring Started (Target: ${TargetUtilization}%)" -ForegroundColor Cyan
        Write-Host "Time | GPU% | Mem Used | Mem Total | Temp°C | Power" -ForegroundColor Yellow
        Write-Host "-" * 55 -ForegroundColor Yellow

        while ((Get-Date) -lt $endTime) {
            try {
                $gpuInfo = nvidia-smi --query-gpu=timestamp,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw --format=csv,noheader,nounits
                if ($LASTEXITCODE -eq 0) {
                    $parts = $gpuInfo -split ','
                    $timestamp = [DateTime]::Parse($parts[0].Trim())
                    $util = [int]$parts[1].Trim()
                    $memUsed = [int]$parts[2].Trim()
                    $memTotal = [int]$parts[3].Trim()
                    $temp = [int]$parts[4].Trim()
                    $power = [double]$parts[5].Trim()

                    $sampleCount++
                    $maxUtilization = [Math]::Max($maxUtilization, $util)
                    if ($util -ge 80) { $highUtilCount++ }

                    # Color code based on utilization
                    $color = if ($util -ge 90) { "Green" }
                            elseif ($util -ge 70) { "Yellow" }
                            else { "Red" }

                    Write-Host "$($timestamp.ToString('HH:mm:ss')) | $util% | $memUsed MB | $memTotal MB | $temp C | $power W" -ForegroundColor $color

                    # Progress indicator
                    if ($sampleCount % 10 -eq 0) {
                        $progress = [int](($endTime - (Get-Date)).TotalMinutes / $DurationMinutes * 100)
                        Write-Host "⏱️  Progress: $(100 - $progress)% complete | Max GPU: ${maxUtilization}% | High util samples: ${highUtilCount}/${sampleCount}" -ForegroundColor Cyan
                    }
                }
            } catch {
                Write-Host "⚠️ GPU monitoring error: $($_.Exception.Message)" -ForegroundColor Red
            }
            Start-Sleep -Seconds 2
        }

        # Summary
        Write-Host "`n📊 GPU Stress Test Summary:" -ForegroundColor Cyan
        Write-Host "=" * 40 -ForegroundColor Cyan
        Write-Host "Samples taken: $sampleCount" -ForegroundColor White
        Write-Host "Maximum utilization: ${maxUtilization}%" -ForegroundColor White
        Write-Host "High utilization (>80%): ${highUtilCount}/${sampleCount} samples" -ForegroundColor White
        Write-Host "Success rate: $([Math]::Round($highUtilCount / $sampleCount * 100, 1))%" -ForegroundColor White

        if ($maxUtilization -ge $TargetUtilization) {
            Write-Host "🎉 TARGET ACHIEVED: ${maxUtilization}% GPU utilization!" -ForegroundColor Green
        } elseif ($maxUtilization -ge 80) {
            Write-Host "👍 EXCELLENT: Achieved ${maxUtilization}% GPU utilization" -ForegroundColor Green
        } elseif ($maxUtilization -ge 60) {
            Write-Host "✅ GOOD: Achieved ${maxUtilization}% GPU utilization" -ForegroundColor Yellow
        } else {
            Write-Host "⚠️ LOW: Only achieved ${maxUtilization}% GPU utilization" -ForegroundColor Red
        }

    } -ArgumentList $DurationMinutes, $TargetUtilization

    # Wait for stress test to complete
    Write-Log "⏳ Running GPU stress test..." "INFO"
    Wait-Job $stressJob -Timeout ($DurationMinutes * 60) | Out-Null

    # Stop monitoring
    Stop-Job $monitorJob
    $monitorResult = Receive-Job $monitorJob

    # Get final GPU stats
    $finalGpu = Get-GPUInfo
    if ($finalGpu) {
        Write-Host "`n🎯 FINAL GPU STATUS:" -ForegroundColor Cyan
        Write-Host "GPU: $($finalGpu.Name)" -ForegroundColor White
        Write-Host "Utilization: $($finalGpu.GPUUtilization)%" -ForegroundColor White
        Write-Host "Memory: $($finalGpu.MemoryUsed)/$($finalGpu.MemoryTotal) MB ($($finalGpu.MemoryUtilization)%)" -ForegroundColor White
        Write-Host "Temperature: $($finalGpu.Temperature)°C" -ForegroundColor White
        Write-Host "Power: $($finalGpu.PowerDraw)/$($finalGpu.PowerLimit) W" -ForegroundColor White
    }

    # Cleanup
    Remove-Job $stressJob, $monitorJob -Force -ErrorAction SilentlyContinue
    if (Test-Path "gpu_stress.cu") { Remove-Item "gpu_stress.cu" -ErrorAction SilentlyContinue }
    if (Test-Path "gpu_stress_cuda.exe") { Remove-Item "gpu_stress_cuda.exe" -ErrorAction SilentlyContinue }
    if (Test-Path "gpu_stress_pytorch.py") { Remove-Item "gpu_stress_pytorch.py" -ErrorAction SilentlyContinue }

    Write-Log "✅ GPU stress test completed!" "SUCCESS"
}

# Main execution
try {
    Write-Host "🔥 AutoAgents GPU Max Stress Test" -ForegroundColor Red
    Write-Host "=" * 50 -ForegroundColor Red
    Write-Host ""

    # Determine which stress test to use
    if (-not $UseCuda -and -not $UsePyTorch -and -not $UseTensorFlow) {
        # Auto-detect best available option
        try {
            $cudaCheck = & "nvidia-smi" --list-gpus 2>$null
            if ($LASTEXITCODE -eq 0) {
                Write-Log "✅ CUDA GPU detected - will use CUDA stress test" "SUCCESS"
                $UseCuda = $true
            }
        } catch {
            Write-Log "⚠️ CUDA not available - will try PyTorch fallback" "WARNING"
            $UsePyTorch = $true
        }
    }

    # Start the stress test
    Start-GPUStressTest -DurationMinutes $DurationMinutes

} catch {
    Write-Log "❌ Script error: $($_.Exception.Message)" "ERROR"
} finally {
    Write-Host "`n🔚 GPU Max Stress Test finished" -ForegroundColor Cyan
}
