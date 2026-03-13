import type { Holding, HoldingWithPrice, PortfolioSnapshot } from '../types/portfolio';

const USD_CAD = 1.36;
const EUR_CAD = 1.47;

const RAW_HOLDINGS: Array<Omit<HoldingWithPrice, 'weight'> & { weight: number }> = [
  // Stocks
  {
    id: '1', symbol: 'AAPL', name: 'Apple Inc.', assetType: 'stock',
    quantity: 50, costBasis: 155.00, currency: 'USD',
    currentPrice: 189.84, currentPriceCad: 189.84 * USD_CAD,
    marketValueCad: 50 * 189.84 * USD_CAD,
    costValueCad: 50 * 155.00 * USD_CAD,
    gainLoss: 50 * (189.84 - 155.00) * USD_CAD,
    gainLossPercent: ((189.84 - 155.00) / 155.00) * 100,
    dailyChangePercent: 1.24,
    weight: 0, createdAt: '2023-01-15T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  {
    id: '2', symbol: 'MSFT', name: 'Microsoft Corp.', assetType: 'stock',
    quantity: 30, costBasis: 310.00, currency: 'USD',
    currentPrice: 415.52, currentPriceCad: 415.52 * USD_CAD,
    marketValueCad: 30 * 415.52 * USD_CAD,
    costValueCad: 30 * 310.00 * USD_CAD,
    gainLoss: 30 * (415.52 - 310.00) * USD_CAD,
    gainLossPercent: ((415.52 - 310.00) / 310.00) * 100,
    dailyChangePercent: -0.83,
    weight: 0, createdAt: '2023-02-01T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  {
    id: '3', symbol: 'NVDA', name: 'NVIDIA Corp.', assetType: 'stock',
    quantity: 25, costBasis: 420.00, currency: 'USD',
    currentPrice: 875.40, currentPriceCad: 875.40 * USD_CAD,
    marketValueCad: 25 * 875.40 * USD_CAD,
    costValueCad: 25 * 420.00 * USD_CAD,
    gainLoss: 25 * (875.40 - 420.00) * USD_CAD,
    gainLossPercent: ((875.40 - 420.00) / 420.00) * 100,
    dailyChangePercent: 3.47,
    weight: 0, createdAt: '2023-03-10T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  {
    id: '4', symbol: 'TD.TO', name: 'Toronto-Dominion Bank', assetType: 'stock',
    quantity: 150, costBasis: 85.00, currency: 'CAD',
    currentPrice: 81.24, currentPriceCad: 81.24,
    marketValueCad: 150 * 81.24,
    costValueCad: 150 * 85.00,
    gainLoss: 150 * (81.24 - 85.00),
    gainLossPercent: ((81.24 - 85.00) / 85.00) * 100,
    dailyChangePercent: -0.42,
    weight: 0, createdAt: '2022-11-01T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  {
    id: '5', symbol: 'RY.TO', name: 'Royal Bank of Canada', assetType: 'stock',
    quantity: 80, costBasis: 120.00, currency: 'CAD',
    currentPrice: 135.88, currentPriceCad: 135.88,
    marketValueCad: 80 * 135.88,
    costValueCad: 80 * 120.00,
    gainLoss: 80 * (135.88 - 120.00),
    gainLossPercent: ((135.88 - 120.00) / 120.00) * 100,
    dailyChangePercent: 0.61,
    weight: 0, createdAt: '2022-12-01T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  // ETFs
  {
    id: '6', symbol: 'VOO', name: 'Vanguard S&P 500 ETF', assetType: 'etf',
    quantity: 40, costBasis: 380.00, currency: 'USD',
    currentPrice: 481.55, currentPriceCad: 481.55 * USD_CAD,
    marketValueCad: 40 * 481.55 * USD_CAD,
    costValueCad: 40 * 380.00 * USD_CAD,
    gainLoss: 40 * (481.55 - 380.00) * USD_CAD,
    gainLossPercent: ((481.55 - 380.00) / 380.00) * 100,
    dailyChangePercent: 0.95,
    weight: 0, createdAt: '2022-06-01T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  {
    id: '7', symbol: 'VFV.TO', name: 'Vanguard S&P 500 Index ETF (CAD)', assetType: 'etf',
    quantity: 200, costBasis: 88.00, currency: 'CAD',
    currentPrice: 112.40, currentPriceCad: 112.40,
    marketValueCad: 200 * 112.40,
    costValueCad: 200 * 88.00,
    gainLoss: 200 * (112.40 - 88.00),
    gainLossPercent: ((112.40 - 88.00) / 88.00) * 100,
    dailyChangePercent: 1.02,
    weight: 0, createdAt: '2022-07-01T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  // Crypto
  {
    id: '8', symbol: 'BTC-CAD', name: 'Bitcoin', assetType: 'crypto',
    quantity: 0.85, costBasis: 42000.00, currency: 'CAD',
    currentPrice: 87450.00, currentPriceCad: 87450.00,
    marketValueCad: 0.85 * 87450.00,
    costValueCad: 0.85 * 42000.00,
    gainLoss: 0.85 * (87450.00 - 42000.00),
    gainLossPercent: ((87450.00 - 42000.00) / 42000.00) * 100,
    dailyChangePercent: 4.82,
    weight: 0, createdAt: '2023-01-20T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  {
    id: '9', symbol: 'ETH-CAD', name: 'Ethereum', assetType: 'crypto',
    quantity: 5.0, costBasis: 2200.00, currency: 'CAD',
    currentPrice: 4380.00, currentPriceCad: 4380.00,
    marketValueCad: 5.0 * 4380.00,
    costValueCad: 5.0 * 2200.00,
    gainLoss: 5.0 * (4380.00 - 2200.00),
    gainLossPercent: ((4380.00 - 2200.00) / 2200.00) * 100,
    dailyChangePercent: -2.14,
    weight: 0, createdAt: '2023-02-10T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  // Cash
  {
    id: '10', symbol: 'USD-CASH', name: 'US Dollar Cash', assetType: 'cash',
    quantity: 8500, costBasis: 1.0, currency: 'USD',
    currentPrice: 1.0, currentPriceCad: USD_CAD,
    marketValueCad: 8500 * USD_CAD,
    costValueCad: 8500 * USD_CAD,
    gainLoss: 0,
    gainLossPercent: 0,
    dailyChangePercent: 0,
    weight: 0, createdAt: '2023-06-01T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  {
    id: '11', symbol: 'EUR-CASH', name: 'Euro Cash', assetType: 'cash',
    quantity: 3000, costBasis: 1.0, currency: 'EUR',
    currentPrice: 1.0, currentPriceCad: EUR_CAD,
    marketValueCad: 3000 * EUR_CAD,
    costValueCad: 3000 * EUR_CAD,
    gainLoss: 0,
    gainLossPercent: 0,
    dailyChangePercent: 0,
    weight: 0, createdAt: '2023-06-01T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
  {
    id: '12', symbol: 'CAD-CASH', name: 'Canadian Dollar Cash', assetType: 'cash',
    quantity: 12500, costBasis: 1.0, currency: 'CAD',
    currentPrice: 1.0, currentPriceCad: 1.0,
    marketValueCad: 12500,
    costValueCad: 12500,
    gainLoss: 0,
    gainLossPercent: 0,
    dailyChangePercent: 0,
    weight: 0, createdAt: '2023-06-01T10:00:00Z', updatedAt: '2024-01-15T10:00:00Z',
  },
];

function buildSnapshot(): PortfolioSnapshot {
  const totalValue = RAW_HOLDINGS.reduce((sum, h) => sum + h.marketValueCad, 0);
  const totalCost = RAW_HOLDINGS.reduce((sum, h) => sum + h.costValueCad, 0);

  const holdings: HoldingWithPrice[] = RAW_HOLDINGS.map((h) => ({
    ...h,
    weight: (h.marketValueCad / totalValue) * 100,
  }));

  const totalGainLoss = totalValue - totalCost;
  const totalGainLossPercent = (totalGainLoss / totalCost) * 100;

  const dailyPnl = holdings.reduce(
    (sum, h) => sum + h.marketValueCad * (h.dailyChangePercent / 100),
    0
  );

  return {
    holdings,
    totalValue,
    totalCost,
    totalGainLoss,
    totalGainLossPercent,
    dailyPnl,
    lastUpdated: new Date().toISOString(),
  };
}

export const MOCK_SNAPSHOT: PortfolioSnapshot = buildSnapshot();

export const MOCK_HOLDINGS: Holding[] = RAW_HOLDINGS.map(
  ({ currentPrice, currentPriceCad, marketValueCad, costValueCad, gainLoss, gainLossPercent, weight, dailyChangePercent, ...h }) => h
);
