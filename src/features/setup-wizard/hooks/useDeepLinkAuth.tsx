import { onOpenUrl } from '@tauri-apps/plugin-deep-link'
import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

export const useDeepLinkAuth = () => {
  const [isHandlingAuth, setIsHandlingAuth] = useState(false)

  useEffect(() => {
    // Check URL parameters immediately
    const urlObj = new URL(window.location.href)
    const searchParams = new URLSearchParams(urlObj.search)
    const code = searchParams.get('code')
    const serviceType = searchParams.get('state') // OAuth state parameter contains serviceType

    const handleAuthCode = async (code: string, serviceType: string) => {
      try {
        await invoke('handle_oauth_callback', { code, serviceType })
        window.postMessage({ type: 'oauth_callback', code, serviceType }, '*')
      } catch (error) {
        console.error('Failed to handle OAuth callback:', error)
        window.postMessage({ type: 'oauth_error', error }, '*')
      }
    }

    if (code && serviceType) {
      handleAuthCode(code, serviceType)
    }

    const handleUrl = async (urls: string[]) => {
      try {
        setIsHandlingAuth(true)
        const url = urls[0]

        const urlObj = new URL(url)
        const searchParams = new URLSearchParams(urlObj.search.substring(1))

        const code = searchParams.get('code')
        const serviceType = searchParams.get('state')

        if (code && serviceType) {
          await handleAuthCode(code, serviceType)
          return
        }
      } catch (err) {
        console.error('Error handling deep link:', err)
        window.postMessage({ type: 'oauth_error', error: err }, '*')
      } finally {
        setIsHandlingAuth(false)
      }
    }

    onOpenUrl(handleUrl)
  }, [])

  return isHandlingAuth
}
