const puppeteer = require('puppeteer');
const path = require('path');

async function testBalance() {
    const browser = await puppeteer.launch({
        headless: true,
        args: ['--no-sandbox', '--disable-setuid-sandbox']
    });
    
    try {
        const page = await browser.newPage();
        
        // Enable console logging
        page.on('console', msg => {
            const type = msg.type();
            const text = msg.text();
            if (type === 'error') {
                console.error('Browser Error:', text);
            } else {
                console.log(`Browser ${type}:`, text);
            }
        });
        
        page.on('pageerror', error => {
            console.error('Page error:', error.message);
        });
        
        // Navigate to the test page via HTTP server
        const url = 'http://localhost:8080/test-balance-testnet.html';
        console.log('Loading page:', url);
        await page.goto(url, { waitUntil: 'networkidle0' });
        
        // Wait for WASM to initialize
        await new Promise(resolve => setTimeout(resolve, 2000));
        
        // Click the fetch button
        console.log('Clicking fetch button...');
        await page.click('#fetchButton');
        
        // Wait for the output to appear
        await page.waitForSelector('#output .success, #output .error', { timeout: 60000 });
        
        // Get the output
        const output = await page.evaluate(() => {
            return document.getElementById('output').textContent;
        });
        
        console.log('\n=== Output from browser ===');
        console.log(output);
        console.log('=========================\n');
        
        // Check if we got the balance
        const balanceMatch = output.match(/Balance: (\d+) credits/);
        const dashMatch = output.match(/Balance in DASH: ([\d.]+) DASH/);
        
        if (balanceMatch && dashMatch) {
            console.log('✓ SUCCESS! Identity balance retrieved:');
            console.log(`  Credits: ${balanceMatch[1]}`);
            console.log(`  DASH: ${dashMatch[1]}`);
        } else {
            console.log('✗ Failed to retrieve balance');
        }
        
    } catch (error) {
        console.error('Error:', error);
    } finally {
        await browser.close();
    }
}

testBalance();