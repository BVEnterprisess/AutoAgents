@echo off
REM AutoAgents GPU Max Performance Startup Script
REM Builds with CUDA support and starts autonomous sync

echo ========================================
echo AutoAgents GPU Max Performance Startup
echo ========================================

REM Set CUDA environment variables for maximum performance
set CUDA_VISIBLE_DEVICES=0
set TORCH_USE_CUDA_DSA=1
set CUDA_LAUNCH_BLOCKING=0

REM Change to project directory
cd /d "%~dp0"

echo Building AutoAgents with CUDA GPU acceleration...
echo.

REM Build liquid-edge with CUDA support
cd crates\liquid-edge
C:\Users\Administrator\.cargo\bin\cargo build --release --features cuda
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Failed to build liquid-edge with CUDA support
    pause
    exit /b 1
)
cd ..\..

REM Build main project
C:\Users\Administrator\.cargo\bin\cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Failed to build main project
    pause
    exit /b 1
)

echo.
echo ========================================
echo Build completed successfully!
echo Starting autonomous Git sync service...
echo ========================================
echo.

REM Start the autonomous sync service in background
start "AutoAgents Sync Service" powershell.exe -ExecutionPolicy Bypass -File "auto_sync.ps1"

echo.
echo ========================================
echo GPU Acceleration Status:
echo ========================================

REM Check GPU status
nvidia-smi --query-gpu=name,memory.used,memory.total,temperature.gpu,utilization.gpu --format=csv,noheader,nounits
if %ERRORLEVEL% NEQ 0 (
    echo No NVIDIA GPU detected or nvidia-smi not available
    echo Using CPU mode
) else (
    echo GPU detected and ready for maximum utilization!
)

echo.
echo ========================================
echo AutoAgents is now running with:
echo - CUDA GPU acceleration enabled
echo - Autonomous Git sync every 10 minutes
echo - Maximum performance optimization
echo ========================================
echo.
echo Press any key to exit (service will continue running)...
pause > nul
