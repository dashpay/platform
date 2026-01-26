export const toHexString = (byteArray: ArrayLike<number>): string =>
  Array.prototype.map
    .call(byteArray, (byte: number) => `0${(byte & 0xff).toString(16)}`.slice(-2))
    .join('');

export const fromHexString = (str: string): Uint8Array => {
  const bytes: number[] = [];
  for (let i = 0; i < str.length; i += 2) {
    bytes.push(parseInt(str.slice(i, i + 2), 16));
  }

  return Uint8Array.from(bytes);
};
