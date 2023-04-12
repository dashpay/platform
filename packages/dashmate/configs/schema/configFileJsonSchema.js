module.exports = {
  $schema: 'https://schema.dash.org/dpp-0-4-0/meta/data-contract#',
  type: 'object',
  properties: {
    configFormatVersion: {
      type: 'string',
    },
    defaultConfigName: {
      type: ['string', 'null'],
    },
    defaultGroupName: {
      type: ['string', 'null'],
    },
    configs: {
      type: 'object',
    },
  },
  required: ['configFormatVersion', 'defaultConfigName', 'defaultGroupName', 'configs'],
  additionalProperties: false,
};
