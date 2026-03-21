import { createContext, useContext } from 'react';

export const SUPPORTED_CURRENCIES = ['CAD', 'USD', 'EUR', 'GBP', 'JPY', 'CHF', 'AUD'] as const;
export type SupportedCurrency = (typeof SUPPORTED_CURRENCIES)[number];

interface CurrencyContextValue {
  baseCurrency: string;
  setBaseCurrency: (currency: string) => void;
}

export const CurrencyContext = createContext<CurrencyContextValue>({
  baseCurrency: 'CAD',
  setBaseCurrency: () => {},
});

export function useCurrency(): CurrencyContextValue {
  return useContext(CurrencyContext);
}
