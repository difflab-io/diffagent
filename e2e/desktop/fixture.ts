import { mount } from 'svelte'
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import { emit } from '@tauri-apps/api/event'
import Shell from '../../apps/diffagent-web/src/lib/components/Shell.svelte'
import '../../apps/diffagent-web/src/app.css'

// Included only in the isolated test build; never included in the production application bundle.
const testWindow = window as unknown as {
  desktopTestInvoke: (command: string, args: unknown) => Promise<unknown>
  desktopTestEmit: (event: string, payload?: unknown) => Promise<void>
  isTauri: boolean
}
mockWindows('main')
mockIPC((command, args) => testWindow.desktopTestInvoke(command, args), {
  shouldMockEvents: true,
})
testWindow.isTauri = true
testWindow.desktopTestEmit = (event, payload) => emit(event, payload)
mount(Shell, { target: document.querySelector('#app')! })
