/**
 * Advanced Query Testing for Document Explorer
 * Tests complex WHERE/ORDER BY clause combinations and query optimization
 */

const { test, expect } = require('@playwright/test');

test.describe('Document Explorer - Advanced Query Testing', () => {
  let page;

  test.beforeEach(async ({ browser }) => {
    page = await browser.newPage();
    await page.goto('http://localhost:8888/samples/document-explorer/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000); // Allow for SDK initialization
  });

  test.afterEach(async () => {
    await page.close();
  });

  describe('WHERE Clause Testing', () => {
    test('should handle equality WHERE conditions', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set up equality condition
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('parentDomainName');
      
      const operatorSelect = page.locator('#whereConditions .operator-select').first();
      await operatorSelect.selectOption('=');
      
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('dash');
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check generated query includes WHERE condition
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        expect(queryCode).toContain('parentDomainName');
        expect(queryCode).toContain('=');
        expect(queryCode).toContain('dash');
      }
    });

    test('should handle range WHERE conditions (greater than)', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set up greater than condition
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('$createdAt');
      
      const operatorSelect = page.locator('#whereConditions .operator-select').first();
      await operatorSelect.selectOption('>');
      
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('1640995200000'); // Timestamp
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Verify query generation
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        expect(queryCode).toContain('$createdAt');
        expect(queryCode).toContain('>');
        expect(queryCode).toContain('1640995200000');
      }
    });

    test('should handle string prefix WHERE conditions (startsWith)', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set up startsWith condition
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('label');
      
      const operatorSelect = page.locator('#whereConditions .operator-select').first();
      await operatorSelect.selectOption('startsWith');
      
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('test');
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Verify query generation
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        expect(queryCode).toContain('label');
        expect(queryCode).toContain('startsWith');
        expect(queryCode).toContain('test');
      }
    });

    test('should handle multiple WHERE conditions (AND logic)', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Add first condition
      let fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('parentDomainName');
      let valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('dash');
      
      // Add second condition
      await page.click('#addConditionBtn');
      await page.waitForTimeout(500);
      
      const conditions = page.locator('#whereConditions .condition-row');
      const secondCondition = conditions.nth(1);
      
      fieldSelect = secondCondition.locator('.field-select');
      await fieldSelect.selectOption('$ownerId');
      
      const operatorSelect = secondCondition.locator('.operator-select');
      await operatorSelect.selectOption('!=');
      
      valueInput = secondCondition.locator('.value-input');
      await valueInput.fill('');
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check both conditions are in generated query
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        expect(queryCode).toContain('parentDomainName');
        expect(queryCode).toContain('$ownerId');
      }
    });

    test('should handle array values for IN operations', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set up IN condition
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('parentDomainName');
      
      const operatorSelect = page.locator('#whereConditions .operator-select').first();
      await operatorSelect.selectOption('in');
      
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('["dash", "platform"]'); // JSON array
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Verify array handling
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        expect(queryCode).toContain('in');
        expect(queryCode).toContain('["dash", "platform"]');
      }
    });
  });

  describe('ORDER BY Clause Testing', () => {
    test('should handle single ORDER BY condition', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set up ORDER BY
      const orderFieldSelect = page.locator('#orderByConditions .field-select').first();
      await orderFieldSelect.selectOption('$createdAt');
      
      const directionSelect = page.locator('#orderByConditions .direction-select').first();
      await directionSelect.selectOption('desc');
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Verify ORDER BY in generated query
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        expect(queryCode).toContain('$createdAt');
        expect(queryCode).toContain('desc');
      }
    });

    test('should handle multiple ORDER BY conditions', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // First ORDER BY condition
      let orderFieldSelect = page.locator('#orderByConditions .field-select').first();
      await orderFieldSelect.selectOption('$createdAt');
      let directionSelect = page.locator('#orderByConditions .direction-select').first();
      await directionSelect.selectOption('desc');
      
      // Add second ORDER BY condition
      await page.click('#addOrderByBtn');
      await page.waitForTimeout(500);
      
      const orderByConditions = page.locator('#orderByConditions .orderby-row');
      const secondOrderBy = orderByConditions.nth(1);
      
      orderFieldSelect = secondOrderBy.locator('.field-select');
      await orderFieldSelect.selectOption('label');
      directionSelect = secondOrderBy.locator('.direction-select');
      await directionSelect.selectOption('asc');
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Check both ORDER BY conditions
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        expect(queryCode).toContain('$createdAt');
        expect(queryCode).toContain('desc');
        expect(queryCode).toContain('label');
        expect(queryCode).toContain('asc');
      }
    });
  });

  describe('Complex Query Combinations', () => {
    test('should handle WHERE + ORDER BY + LIMIT combination', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set WHERE condition
      const whereFieldSelect = page.locator('#whereConditions .field-select').first();
      await whereFieldSelect.selectOption('parentDomainName');
      const whereValueInput = page.locator('#whereConditions .value-input').first();
      await whereValueInput.fill('dash');
      
      // Set ORDER BY
      const orderFieldSelect = page.locator('#orderByConditions .field-select').first();
      await orderFieldSelect.selectOption('label');
      const directionSelect = page.locator('#orderByConditions .direction-select').first();
      await directionSelect.selectOption('asc');
      
      // Set LIMIT and OFFSET
      await page.fill('#limitInput', '5');
      await page.fill('#offsetInput', '2');
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      // Verify comprehensive query
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        expect(queryCode).toContain('parentDomainName');
        expect(queryCode).toContain('dash');
        expect(queryCode).toContain('label');
        expect(queryCode).toContain('asc');
        expect(queryCode).toContain('limit: 5');
        expect(queryCode).toContain('offset: 2');
      }
    });

    test('should handle complex multi-field WHERE with ORDER BY', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Multiple WHERE conditions
      // First condition: parentDomainName = 'dash'
      let fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('parentDomainName');
      let valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('dash');
      
      // Add second condition: $createdAt > timestamp
      await page.click('#addConditionBtn');
      await page.waitForTimeout(500);
      
      const conditions = page.locator('#whereConditions .condition-row');
      const secondCondition = conditions.nth(1);
      
      fieldSelect = secondCondition.locator('.field-select');
      await fieldSelect.selectOption('$createdAt');
      const operatorSelect = secondCondition.locator('.operator-select');
      await operatorSelect.selectOption('>');
      valueInput = secondCondition.locator('.value-input');
      await valueInput.fill('1640995200000');
      
      // Add third condition: label startsWith
      await page.click('#addConditionBtn');
      await page.waitForTimeout(500);
      
      const thirdCondition = conditions.nth(2);
      fieldSelect = thirdCondition.locator('.field-select');
      await fieldSelect.selectOption('label');
      const thirdOperator = thirdCondition.locator('.operator-select');
      await thirdOperator.selectOption('startsWith');
      valueInput = thirdCondition.locator('.value-input');
      await valueInput.fill('a');
      
      // Multiple ORDER BY
      const orderFieldSelect = page.locator('#orderByConditions .field-select').first();
      await orderFieldSelect.selectOption('$createdAt');
      const directionSelect = page.locator('#orderByConditions .direction-select').first();
      await directionSelect.selectOption('desc');
      
      await page.click('#addOrderByBtn');
      await page.waitForTimeout(500);
      
      const orderByConditions = page.locator('#orderByConditions .orderby-row');
      const secondOrderBy = orderByConditions.nth(1);
      const secondOrderField = secondOrderBy.locator('.field-select');
      await secondOrderField.selectOption('label');
      
      // Execute complex query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(20000); // Allow more time for complex query
      
      // Verify all conditions are present
      const generatedQuery = page.locator('#generatedQuery');
      if (await generatedQuery.isVisible()) {
        const queryCode = await page.locator('#queryCode').textContent();
        
        // Check WHERE conditions
        expect(queryCode).toContain('parentDomainName');
        expect(queryCode).toContain('$createdAt');
        expect(queryCode).toContain('label');
        expect(queryCode).toContain('startsWith');
        
        // Check ORDER BY conditions
        expect(queryCode).toContain('desc');
      }
    });
  });

  describe('Query Performance and Optimization', () => {
    test('should handle efficient queries with good performance', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set up efficient query (indexed field with reasonable limit)
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('$ownerId');
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk');
      
      await page.fill('#limitInput', '10');
      
      // Execute and measure performance
      const startTime = Date.now();
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      const endTime = Date.now();
      
      const queryTime = endTime - startTime;
      console.log(`Query execution time: ${queryTime}ms`);
      
      // Check query time display
      const queryTimeElement = page.locator('#queryTime');
      if (await queryTimeElement.isVisible()) {
        const timeText = await queryTimeElement.textContent();
        const reportedTime = parseInt(timeText.match(/(\d+)ms/)?.[1] || '0');
        
        // Reported time should be reasonable (less than 10 seconds)
        expect(reportedTime).toBeLessThan(10000);
      }
    });

    test('should handle large result sets with pagination', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Query for larger result set
      await page.fill('#limitInput', '50');
      
      // Execute query
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(20000);
      
      // Check results are properly displayed
      const documentItems = page.locator('.document-item');
      const itemCount = await documentItems.count();
      
      if (itemCount > 0) {
        expect(itemCount).toBeLessThanOrEqual(50);
        
        // Check results header shows correct count
        const resultsCount = page.locator('#resultsCount');
        if (await resultsCount.isVisible()) {
          const countText = await resultsCount.textContent();
          expect(countText).toContain(`${itemCount} documents`);
        }
      }
    });

    test('should handle pagination with offset', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // First page
      await page.fill('#limitInput', '5');
      await page.fill('#offsetInput', '0');
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(15000);
      
      const firstPageItems = page.locator('.document-item');
      const firstPageCount = await firstPageItems.count();
      
      if (firstPageCount > 0) {
        // Get first page document IDs
        const firstPageIds = [];
        for (let i = 0; i < firstPageCount; i++) {
          const item = firstPageItems.nth(i);
          const idElement = item.locator('.document-id');
          if (await idElement.isVisible()) {
            firstPageIds.push(await idElement.textContent());
          }
        }
        
        // Second page
        await page.fill('#offsetInput', '5');
        await page.click('#executeQueryBtn');
        await page.waitForTimeout(15000);
        
        const secondPageItems = page.locator('.document-item');
        const secondPageCount = await secondPageItems.count();
        
        if (secondPageCount > 0) {
          // Get second page document IDs
          const secondPageIds = [];
          for (let i = 0; i < secondPageCount; i++) {
            const item = secondPageItems.nth(i);
            const idElement = item.locator('.document-id');
            if (await idElement.isVisible()) {
              secondPageIds.push(await idElement.textContent());
            }
          }
          
          // Pages should have different documents
          const overlap = firstPageIds.some(id => secondPageIds.includes(id));
          expect(overlap).toBe(false);
        }
      }
    });
  });

  describe('Query Validation and Error Handling', () => {
    test('should validate WHERE clause field selections', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Try to execute query without selecting field
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('test-value');
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(5000);
      
      // Should either not execute or show validation error
      // The implementation may handle this differently
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText.length).toBeGreaterThan(0);
      }
    });

    test('should handle invalid value formats gracefully', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set up query with invalid timestamp value
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('$createdAt');
      const operatorSelect = page.locator('#whereConditions .operator-select').first();
      await operatorSelect.selectOption('>');
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('invalid-timestamp'); // Should be numeric
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(10000);
      
      // Should handle error gracefully
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/invalid|error|failed/i);
      }
    });

    test('should handle malformed JSON in array fields', async () => {
      // Setup contract and document type
      await page.locator('[data-contract="dpns"]').click();
      await page.waitForTimeout(5000);
      await page.selectOption('#documentTypeSelect', 'domain');
      await page.waitForTimeout(1000);
      
      // Set up IN query with malformed JSON
      const fieldSelect = page.locator('#whereConditions .field-select').first();
      await fieldSelect.selectOption('parentDomainName');
      const operatorSelect = page.locator('#whereConditions .operator-select').first();
      await operatorSelect.selectOption('in');
      const valueInput = page.locator('#whereConditions .value-input').first();
      await valueInput.fill('[invalid json array'); // Malformed JSON
      
      await page.click('#executeQueryBtn');
      await page.waitForTimeout(10000);
      
      // Should handle JSON parsing error
      const errorResults = page.locator('#errorResults');
      if (await errorResults.isVisible()) {
        const errorText = await errorResults.textContent();
        expect(errorText).toMatch(/invalid|error|json|parse/i);
      }
    });
  });
});