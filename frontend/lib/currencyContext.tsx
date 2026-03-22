import { createContext, useContext } from 'react';

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
