export default {
  $schema: 'http://json-schema.org/draft-07/schema#',
  type: 'object',
  properties: {
    configFormatVersion: {
      type: 'string',
    },
    defaultConfigName: {
      type: ['string', 'null'],
      pattern: '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$',
    },
    defaultGroupName: {
      type: ['string', 'null'],
      pattern: '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$',
    },
    configs: {
      type: 'object',
      propertyNames: {
        pattern: '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$',
      },
    },
    projectId: {
      type: 'string',
      pattern: '^[a-f0-9]{8}$',
    },
  },
  required: ['configFormatVersion', 'defaultConfigName', 'defaultGroupName', 'configs'],
  additionalProperties: false,
};
