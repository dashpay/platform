module.exports = function getLovelyDapContract() {
  return {
    $schema: 'https://schema.dash.org/dap-0-4-0/meta/dap-contract',
    name: 'lovelyContract',
    version: 1,
    dapObjectsDefinition: {
      niceObject: {
        properties: {
          name: {
            type: 'string',
          },
        },
        additionalProperties: false,
      },
      prettyObject: {
        properties: {
          lastName: {
            $ref: '#/definitions/lastName',
          },
        },
        required: ['lastName'],
        additionalProperties: false,
      },
    },
    definitions: {
      lastName: {
        type: 'string',
      },
    },
  };
};
