"use client"

import { ApiPromise, WsProvider } from '@polkadot/api'
import { createContext, useContext, useState, useEffect, useRef } from 'react'
import { cryptoWaitReady } from '@polkadot/util-crypto';
import { ContractPromise } from '@polkadot/api-contract';
import { Keyring } from '@polkadot/keyring';
import { KeyringPair } from '@polkadot/keyring/types';
import contractMetadata from '../../contract/target/ink/plinko_contract.json'

const PolkadotContext = createContext<{
    idncClient: ApiPromise | null
    alice: KeyringPair | null
    contract: ContractPromise | null
    loading: boolean
}>({
    idncClient: null,
    alice: null,
    contract: null,
    loading: true,
})
const CONTRACT_ADDRESS = "5GA2EU1JX3gHe17UrGaiixNXcjoHGiWaxrMMVGcDHGU4K9wf"

export function PolkadotProvider({ children }: {children: React.ReactNode }) {
    const [idncClient, setClient] = useState<ApiPromise | null>(null)
    const [alice, setAlice] = useState<KeyringPair | null>(null)
    const [contract, setContract] = useState<ContractPromise | null>(null)
    const [loading, setLoading] = useState(true)
    const initializedRef = useRef(false)

    useEffect(() => {
        console.log("useEffect called in Polkadot Provider")
        if (initializedRef.current) {
            console.log("client already initialized")
            return;
        } 

        async function initPolkadotApi() {
            try {
                console.log("initializing client")
                await cryptoWaitReady();
                
                const wsProvider = new WsProvider('ws://127.0.0.1:9944')

                wsProvider.on('connected', () => console.log('WS Connected'))
                wsProvider.on('disconnected', () => {
                    console.log('WS Disconnected')
                    teardownClient();
                })
                wsProvider.on('error', (error) => {
                    console.error('WS Error:', error)
                    teardownClient()
                })

                const idncClient = await ApiPromise.create({ provider: wsProvider })
                await idncClient.isReady
                setClient(idncClient)
                console.log("client initialized")
                const keyring = new Keyring({ type: 'sr25519' });
                const alice = keyring.addFromUri('//Alice', { name: 'Alice' });
                setAlice(alice)
                console.log("Alice keyring derived")
                const contract = new ContractPromise(idncClient, contractMetadata, CONTRACT_ADDRESS);
                setContract(contract)
                initializedRef.current = true
                setLoading(false)
                
            } catch (error) {
                console.error("Failed to initialize PolkadotClient", error)
                teardownClient()
            }
        }
        initPolkadotApi();

        function teardownClient() {
            console.log("tearing down client")
            initializedRef.current = false
            setClient(null)
            setAlice(null)
            setContract(null)
            setLoading(false)
        }

        return () => {
          // Don't disconnect on navigation, only on app close
          // The connection will be reused
        }
    }, [])

    return (
        <PolkadotContext.Provider value = {{idncClient, alice, contract, loading}}>
            {children}
        </PolkadotContext.Provider>
    )
}

export function usePolkadot() {
    console.log("usePolkadot called")
    return useContext(PolkadotContext)
}
