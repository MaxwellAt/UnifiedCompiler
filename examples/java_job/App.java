public class App {
    public static void main(String[] args) {
        System.out.println("============================================================");
        System.out.println("  [JVM CLUSTER JOB] Running on Remote Host");
        System.out.println("  Java Runtime   : " + System.getProperty("java.runtime.name"));
        System.out.println("  Java Version   : " + System.getProperty("java.version"));
        System.out.println("  VM Vendor      : " + System.getProperty("java.vm.vendor"));
        System.out.println("  OS Name / Arch : " + System.getProperty("os.name") + " (" + System.getProperty("os.arch") + ")");
        System.out.println("============================================================");

        long start = System.currentTimeMillis();
        System.out.println("Computing Prime Numbers up to 500,000 using Sieve of Eratosthenes...");

        int limit = 500_000;
        boolean[] isPrime = new boolean[limit + 1];
        for (int i = 2; i <= limit; i++) isPrime[i] = true;

        for (int p = 2; p * p <= limit; p++) {
            if (isPrime[p]) {
                for (int i = p * p; i <= limit; i += p) {
                    isPrime[i] = false;
                }
            }
        }

        int count = 0;
        for (int i = 2; i <= limit; i++) {
            if (isPrime[i]) count++;
        }

        long elapsed = System.currentTimeMillis() - start;
        System.out.println("------------------------------------------------------------");
        System.out.println("  Primes Found   : " + count);
        System.out.println("  Calculation in : " + elapsed + " ms");
        System.out.println("============================================================");
    }
}
