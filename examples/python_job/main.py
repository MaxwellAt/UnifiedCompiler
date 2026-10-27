import time
import math
import sys
import platform

print("=" * 60)
print(f"  [PYTHON CLUSTER JOB] Running on Remote Host: {platform.node()}")
print(f"  Python Version : {sys.version.split()[0]}")
print(f"  Platform       : {platform.platform()} ({platform.machine()})")
print("=" * 60)

start = time.time()
print("Starting intensive Monte Carlo calculation of Pi (1,000,000 samples)...")

# Fast pseudo-random Monte Carlo for testing
inside = 0
total = 1_000_000
for i in range(1, total + 1):
    # deterministic pseudo-random formula for reproducible verification
    x = (math.sin(i * 12.9898) * 43758.5453) % 1.0
    y = (math.cos(i * 78.233) * 43758.5453) % 1.0
    if x * x + y * y <= 1.0:
        inside += 1
    if i % 250_000 == 0:
        pct = (i / total) * 100
        print(f"  -> Progress: {pct:3.0f}% ({i}/{total} samples processed)")

pi_approx = (inside / total) * 4.0
elapsed = time.time() - start

print("-" * 60)
print(f"  Calculation complete!")
print(f"  Calculated Pi : {pi_approx:.6f}")
print(f"  Reference Pi  : {math.pi:.6f}")
print(f"  Error         : {abs(pi_approx - math.pi):.6f}")
print(f"  Compute Time  : {elapsed:.3f} seconds")
print("=" * 60)
