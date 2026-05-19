const varIntSizeBytesFromLength = (length) => {
  let bytes = 1;
  if (length >= 0xfd) {
    bytes += 2;
    if (length >= 0xffff) {
      bytes += 2;
      if (length >= 0xffffffff) {
        bytes += 4;
      }
    }
  }
  return bytes;
};

export { varIntSizeBytesFromLength };
export default { varIntSizeBytesFromLength };
