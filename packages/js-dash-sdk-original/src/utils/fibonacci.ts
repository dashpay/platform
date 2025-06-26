export const fibonacci = (n: number): number => {
  if (n < 2) {
    return n;
  }
  return fibonacci(n - 1) + fibonacci(n - 2);
};

export const nearestGreaterFibonacci = (value: number) => {
  const phi = (1 + Math.sqrt(5)) / 2;

  // Use the rearranged Binet's formula to find the nearest index
  const n = Math.ceil(Math.log(value * Math.sqrt(5) + 0.5) / Math.log(phi));

  // Calculate the Fibonacci number using Binet's formula
  return Math.round(phi ** n / Math.sqrt(5));
};
