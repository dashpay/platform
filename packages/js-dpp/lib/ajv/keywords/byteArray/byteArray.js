const byteArray = {
  keyword: 'byteArray',
  type: ['array'],
  schemaType: [],
  macro(schema, parentSchema) {
    if (parentSchema.items) {
      throw new Error("'byteArray' should not be used with 'items'");
    }

    if (parentSchema.prefixItems) {
      throw new Error("'byteArray' should not be used with 'prefixItems'");
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
