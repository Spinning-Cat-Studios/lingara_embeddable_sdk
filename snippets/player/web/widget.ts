import { defineLingaraEmbed, type LingaraEmbedElement } from '@lingara/embed'

defineLingaraEmbed()
const widget = document.querySelector('lingara-embed') as LingaraEmbedElement

// lingara:begin tokenProvider
// Your own endpoint, on your own server. It mints with this page's origin and
// returns exactly { token, expires_at }: the token (lgr_et_…) never touches a URL.
widget.tokenProvider = async () => {
  const response = await fetch('/lingara/token', { method: 'POST', credentials: 'same-origin' })
  if (!response.ok) throw new Error(`token endpoint answered ${response.status}`)
  return response.json()
}
// lingara:end

// lingara:begin events
widget.addEventListener('lingara:practice-completed', (event) => {
  const { planId, setNumber, correct, total } = (event as CustomEvent).detail
  console.log(`set ${setNumber} of ${planId}: ${correct}/${total}`)
})
widget.addEventListener('lingara:error', (event) => {
  const { code, retryable } = (event as CustomEvent).detail
  // `retryable` means a remount may succeed; when to try is yours to decide.
  if (retryable) setTimeout(() => widget.reload(), 30_000)
  else console.warn('Lingara practice stopped:', code)
})
// lingara:end
