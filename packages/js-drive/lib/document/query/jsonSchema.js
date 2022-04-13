module.exports = {
  type: 'object',
  $defs: {
    field: {
      $id: 'field',
      type: 'string',
      minLength: 1,
      maxLength: 255,
      pattern: '^(\\$id|\\$ownerId|[a-zA-Z0-9-_]|[a-zA-Z0-9-_]+(.[a-zA-Z0-9-_]+)+?)$',
    },
    scalarTypes: {
      $id: 'scalarTypes',
      oneOf: [
        {
          type: 'string',
          maxLength: 1024,
        },
        {
          type: 'number',
        },
        {
          type: 'boolean',
        },
        {
          type: 'object',
          instanceof: 'Buffer',
        },
      ],
    },
  },
  properties: {
    where: {
      $id: 'where',
      type: 'array',
      // Condition
      items: {
        type: 'array',
        oneOf: [
          // Comparisons
          {
            prefixItems: [
              {
                $ref: 'field',
              },
              {
                type: 'string',
                enum: ['<', '<=', '==', '>', '>='],
              },
              {
                $ref: 'scalarTypes',
              },
            ],
            minItems: 3,
            maxItems: 3,
          },
          // Timestamps
          {
            prefixItems: [
              {
                type: 'string',
                enum: ['$createdAt', '$updatedAt'],
              },
              {
                type: 'string',
                enum: ['<', '<=', '==', '>', '>='],
              },
              {
                type: 'integer',
                minimum: 0,
              },
            ],
            minItems: 3,
            maxItems: 3,
          },
          // in
          {
            prefixItems: [
              {
                $ref: 'field',
              },
              {
                type: 'string',
                const: 'in',
              },
              {
                type: 'array',
                items: {
                  $ref: 'scalarTypes',
                },
                uniqueItems: true,
                minItems: 1,
                maxItems: 100,
              },
            ],
            minItems: 3,
            maxItems: 3,
          },
          // startsWith
          {
            prefixItems: [
              {
                $ref: 'field',
              },
              {
                type: 'string',
                const: 'startsWith',
              },
              {
                type: 'string',
                minLength: 1,
                maxLength: 255,
              },
            ],
            minItems: 3,
            maxItems: 3,
          },
        ],
      },
      minItems: 1,
      maxItems: 10,
    },
    limit: {
      type: 'number',
      minimum: 1,
      maximum: 100,
      multipleOf: 1.0,
    },
    orderBy: {
      type: 'array',
      items: {
        type: 'array',
        prefixItems: [
          {
            type: 'string',
            minLength: 1,
            maxLength: 255,
            pattern: '^(\\$id|\\$ownerId|\\$createdAt|\\$updatedAt|[a-zA-Z0-9-_]|[a-zA-Z0-9-_]+(.[a-zA-Z0-9-_]+)+?)$',
          },
          {
            type: 'string',
            enum: ['asc', 'desc'],
          },
        ],
        minItems: 2,
        maxItems: 2,
        items: false,
      },
      minItems: 1,
      maxItems: 255,
    },
    startAfter: {
      type: 'object',
      instanceof: 'Buffer',
    },
    startAt: {
      type: 'object',
      instanceof: 'Buffer',
    },
  },
  dependentSchemas: {
    startAt: {
      not: {
        properties: {
          startAfter: true,
        },
        required: ['startAfter'],
      },
    },
    startAfter: {
      not: {
        properties: {
          startAt: true,
        },
        required: ['startAt'],
      },
    },
  },
  additionalProperties: false,
};
