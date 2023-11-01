import { defineConfig } from 'vite'
import solid from 'vite-plugin-solid'
import { internalIpV4 } from 'internal-ip'

export default defineConfig(async () => {
    const host = await internalIpV4();
    return {
        plugins: [solid()],
        clearScreen: false,
        server: {
            strictPort: true,
            port: 5173,
            host: '0.0.0.0',
            hmr: {
                protocol: 'ws',
                host: '0.0.0.0', 
                port: 5183
            }
        },
        envPrefix: ['VITE_', 'TAURI_PLATFORM', 'TAURI_ARCH', 'TAURI_FAMILY', 'TAURI_PLATFORM_VERSION', 'TAURI_PLATFORM_TYPE', 'TAURI_DEBUG'],
        build: {
            target: process.env.TAURI_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
            minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
            sourcemap: !!process.env.TAURI_DEBUG
        }
    }
})
