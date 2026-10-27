use std::time::Instant;

fn main() {
    println!("============================================================");
    println!("  [RUST CLUSTER JOB] Compiled & Executed Remotely");
    println!("  Target Architecture: {}", std::env::consts::ARCH);
    println!("  Target OS          : {}", std::env::consts::OS);
    println!("============================================================");

    let start = Instant::now();
    println!("Executing high-throughput memory throughput & Collatz conjecture...");

    let mut max_steps = 0;
    let mut num_with_max = 0;

    for n in 1..=500_000u64 {
        let mut val = n;
        let mut steps = 0;
        while val != 1 {
            if val % 2 == 0 {
                val /= 2;
            } else {
                val = val * 3 + 1;
            }
            steps += 1;
        }

        if steps > max_steps {
            max_steps = steps;
            num_with_max = n;
        }
    }

    let elapsed = start.elapsed();
    println!("------------------------------------------------------------");
    println!("  Longest Collatz sequence below 500,000:");
    println!("  Number : {}", num_with_max);
    println!("  Steps  : {}", max_steps);
    println!("  Runtime: {:.3?}", elapsed);
    println!("============================================================");
}
