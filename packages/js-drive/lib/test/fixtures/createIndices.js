/**
 * @param {number} count
 * @param {boolean} [unique=false]
 */
function createIndices(count, unique = false) {
  const indices = [];

  const indexCount = (count < 10 ? count : 10);

  let propertyIndex = 0;

  const basePropertyCount = Math.floor(count / indexCount);
  const propertyLeftovers = count % indexCount;

  for (let i = 0; i < indexCount; i++) {
    const properties = [];

    for (let x = 0; x < basePropertyCount + ((i < propertyLeftovers) ? 1 : 0); x++) {
      const name = `property${propertyIndex}`;

      propertyIndex++;

      properties.push({ [name]: 'asc' });
    }

    indices.push({
      name: `index${i}`,
      properties,
      unique: unique && i < 3,
    });
  }

  return indices;
}

module.exports = createIndices;
