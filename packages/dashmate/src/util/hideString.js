export default function hideString(string) {
  return (typeof string === 'string'
    ? '*'.repeat(string.length)
    : string);
}
