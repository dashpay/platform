# CDN Usage Guide for Dash WASM SDK

The Dash WASM SDK is available through major CDNs for direct browser usage without npm installation.

## Supported CDNs

### unpkg

```html
<!-- Latest version -->
<script type="module">
  import init, * as DashWasm from 'https://unpkg.com/@dashevo/dash-wasm-sdk@latest';
  
  (async () => {
    await init();
    // Use DashWasm functions here
  })();
</script>

<!-- Specific version -->
<script type="module">
  import init, * as DashWasm from 'https://unpkg.com/@dashevo/dash-wasm-sdk@0.1.0';
</script>

<!-- Alpha version -->
<script type="module">
  import init, * as DashWasm from 'https://unpkg.com/@dashevo/dash-wasm-sdk@alpha';
</script>
```

### jsDelivr

```html
<!-- Latest version -->
<script type="module">
  import init, * as DashWasm from 'https://cdn.jsdelivr.net/npm/@dashevo/dash-wasm-sdk@latest';
  
  (async () => {
    await init();
    // Use DashWasm functions here
  })();
</script>

<!-- Specific version -->
<script type="module">
  import init, * as DashWasm from 'https://cdn.jsdelivr.net/npm/@dashevo/dash-wasm-sdk@0.1.0';
</script>

<!-- Alpha version -->
<script type="module">
  import init, * as DashWasm from 'https://cdn.jsdelivr.net/npm/@dashevo/dash-wasm-sdk@alpha';
</script>
```

## Complete HTML Example

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Dash WASM SDK CDN Example</title>
</head>
<body>
    <h1>Dash Platform WASM SDK</h1>
    <div id="output"></div>

    <script type="module">
        import init, * as DashWasm from 'https://unpkg.com/@dashevo/dash-wasm-sdk@latest';
        
        async function loadSDK() {
            try {
                // Initialize the WASM module
                await init();
                
                console.log('✅ Dash WASM SDK loaded successfully');
                document.getElementById('output').innerHTML = '✅ SDK loaded successfully!';
                
                // Example: Create an identity (placeholder - actual implementation depends on SDK)
                // const identity = DashWasm.create_identity();
                // console.log('Identity created:', identity);
                
            } catch (error) {
                console.error('❌ Failed to load SDK:', error);
                document.getElementById('output').innerHTML = '❌ Failed to load SDK';
            }
        }
        
        loadSDK();
    </script>
</body>
</html>
```

## CDN URLs Reference

### unpkg URLs

- **Latest**: `https://unpkg.com/@dashevo/dash-wasm-sdk@latest`
- **Alpha**: `https://unpkg.com/@dashevo/dash-wasm-sdk@alpha`
- **Beta**: `https://unpkg.com/@dashevo/dash-wasm-sdk@beta`
- **Specific version**: `https://unpkg.com/@dashevo/dash-wasm-sdk@0.1.0`

### jsDelivr URLs

- **Latest**: `https://cdn.jsdelivr.net/npm/@dashevo/dash-wasm-sdk@latest`
- **Alpha**: `https://cdn.jsdelivr.net/npm/@dashevo/dash-wasm-sdk@alpha`
- **Beta**: `https://cdn.jsdelivr.net/npm/@dashevo/dash-wasm-sdk@beta`
- **Specific version**: `https://cdn.jsdelivr.net/npm/@dashevo/dash-wasm-sdk@0.1.0`

## Important Notes

### WASM Initialization

⚠️ **Always initialize the WASM module before using SDK functions:**

```javascript
import init, * as DashWasm from 'https://unpkg.com/@dashevo/dash-wasm-sdk@latest';

// REQUIRED: Initialize before use
await init();

// Now you can use SDK functions
// const result = DashWasm.some_function();
```

### CORS Considerations

If you encounter CORS issues, ensure your web server serves the content with appropriate headers, or use a CDN that provides CORS headers.

### Performance Recommendations

1. **Pin to specific versions** in production to avoid unexpected updates
2. **Use SRI (Subresource Integrity)** for enhanced security:

```html
<script type="module" 
        integrity="sha384-..." 
        crossorigin="anonymous"
        src="https://unpkg.com/@dashevo/dash-wasm-sdk@0.1.0">
</script>
```

3. **Preload for better performance**:

```html
<link rel="modulepreload" href="https://unpkg.com/@dashevo/dash-wasm-sdk@0.1.0">
```

### Browser Support

The WASM SDK requires:
- Modern browsers with WebAssembly support
- ES modules support
- For older browsers, consider using a module bundler

### Debugging

Enable detailed logging in development:

```javascript
import init, * as DashWasm from 'https://unpkg.com/@dashevo/dash-wasm-sdk@latest';

// Enable debug logging if available
if (DashWasm.set_debug) {
    DashWasm.set_debug(true);
}

await init();
```

## CDN Availability Monitoring

Both unpkg and jsDelivr provide status pages:

- **unpkg**: https://status.unpkg.com/
- **jsDelivr**: https://www.jsdelivr.com/status

## Support

If you encounter CDN-related issues:

1. Verify the package exists on npm: https://www.npmjs.com/package/@dashevo/dash-wasm-sdk
2. Check CDN status pages
3. Try alternative CDN providers
4. Report issues at: https://github.com/dashpay/platform/issues

## Migration from npm to CDN

If migrating from npm installation to CDN:

### Before (npm):
```javascript
import init, * as DashWasm from '@dashevo/dash-wasm-sdk';
```

### After (CDN):
```javascript
import init, * as DashWasm from 'https://unpkg.com/@dashevo/dash-wasm-sdk@latest';
```

The API remains the same, only the import source changes.