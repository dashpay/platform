export default function validatePositiveInteger(value) {
  const index = Math.floor(Number(value));

  return index >= 0 && index.toString() === value;
}
