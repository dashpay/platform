module.exports = {
  type: 'object',
  definitions: {
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
          maxLength: 512,
        },
        {
          type: 'number',
        },
        {
          type: 'boolean',
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
            items: [
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
          },
          // in
          {
            items: [
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
          },
          // startsWith
          {
            items: [
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
          },
          // elementMatch
          {
            items: [
              {
                $ref: 'field',
              },
              {
                type: 'string',
                const: 'elementMatch',
              },
              {
                allOf: [
                  {
                    $ref: 'where',
                  },
                  {
                    type: 'array',
                    minItems: 2,
                  },
                ],
              },
            ],
          },
          // length
          {
            items: [
              {
                $ref: 'field',
              },
              {
                type: 'string',
                const: 'length',
              },
              {
                type: 'number',
                minimum: 0,
                multipleOf: 1.0,
              },
            ],
          },
          // contains
          {
            items: [
              {
                $ref: 'field',
              },
              {
                type: 'string',
                const: 'contains',
              },
              {
                oneOf: [
                  {
                    $ref: 'scalarTypes',
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
              },
            ],
          },
        ],
        additionalItems: false,
        minItems: 3,
        maxItems: 3,
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
        items: [
          {
            $ref: 'field',
          },
          {
            type: 'string',
            enum: ['asc', 'desc'],
          },
        ],
        minItems: 2,
        maxItems: 2,
        additionalItems: false,
      },
      minItems: 1,
      maxItems: 2,
    },
    startAfter: {
      type: 'number',
      minimum: 1,
      maximum: 20000,
      multipleOf: 1.0,
    },
    startAt: {
      type: 'number',
      minimum: 1,
      maximum: 20000,
      multipleOf: 1.0,
    },
  },
  anyOf: [
    {
      required: ['startAt'],
      not: {
        required: ['startAfter'],
      },
    },
    {
      required: ['startAfter'],
      not: {
        required: ['startAt'],
      },
    },
    {
      not: {
        required: ['startAt', 'startAfter'],
      },
    },
  ],
  additionalProperties: false,
};
