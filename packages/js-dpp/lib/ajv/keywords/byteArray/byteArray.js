const byteArray = {
  type: 'array',
  macro(schema, parentSchema) {
    if (parentSchema.items) {
      throw new Error("'byteArray' should not be used with 'items'");
    }

    return {
      items: {
        type: 'integer',
        minimum: 0,
        maximum: 255,
      },
    };
  },
  errors: false,
  metaSchema: {
    type: 'boolean',
    const: true,
  },
};

module.exports = byteArray;
