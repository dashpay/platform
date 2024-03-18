export default {
  $schema: 'http://json-schema.org/draft-07/schema#',
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
    projectId: {
      type: 'string',
      pattern: '^[a-f0-9]{8}$',
    },
  },
  required: ['configFormatVersion', 'defaultConfigName', 'defaultGroupName', 'configs'],
  additionalProperties: false,
};
