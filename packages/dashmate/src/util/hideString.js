export default function hideString(string) {
  return (typeof string === 'string'
    ? Array.from(string).map(() => '*').join('')
    : string);
}
