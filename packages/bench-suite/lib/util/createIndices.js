/**
 * @param {number} count
 * @param {boolean} [unique=false]
 */
function createIndices(count, unique = false) {
  const indices = [];

  for (let i = 0; i < count; i++) {
    const name = `property${i}`;

    indices.push({
      name: `index${i}`,
      properties: [{ [name]: 'asc' }],
      unique: unique && count < 3,
    });
  }

  return indices;
}

module.exports = createIndices;
