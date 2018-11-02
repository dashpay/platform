module.exports = function getLovelyContract() {
  return {
    $schema: 'https://schema.dash.org/platform-4-0-0/system/meta/dap-contract',
    name: 'lovelyContract',
    version: 1,
    objectsDefinition: {
      niceObject: {
        allOf: [
          { $ref: 'https://schema.dash.org/platform-4-0-0/system/base/dap-object' },
        ],
        properties: {
          name: {
            type: 'string',
          },
        },
      },
      prettyObject: {
        allOf: [
          { $ref: 'https://schema.dash.org/platform-4-0-0/system/base/dap-object' },
        ],
        properties: {
          lastName: {
            type: 'string',
          },
        },
        required: ['lastName'],
      },
    },
  };
};
