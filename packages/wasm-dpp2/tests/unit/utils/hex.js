export const toHexString = (byteArray) => Array.prototype.map.call(byteArray, (byte) => byte.toString(16).padStart(2, '0')).join('');

export const fromHexString = (str) => {
  const bytes = [];
  for (let i = 0; i < str.length; i += 2) {
    bytes.push(parseInt(str.slice(i, i + 2), 16));
  }

  return Uint8Array.from(bytes);
};
