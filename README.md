# MVP QR Recognition

🔍 Надёжный, масштабируемый модуль распознавания QR-кодов с компиляцией в WebAssembly.

## Возможности

### Распознавание в сложных условиях
- ✅ Наклон и перспективное искажение
- ✅ Частичное перекрытие
- ✅ Блики и отражения
- ✅ Низкое и неравномерное освещение
- ✅ Размытие и шум
- ✅ Низкое разрешение камеры

### Функционал
- 📸 Обнаружение нескольких QR-кодов в кадре
- 💳 Распознавание платёжных QR (EMV, СБП, ST.00012)
- 🎯 Определение наиболее релевантного QR для оплаты
- ⚡ Предобработка изображений в реальном времени

### Технологии
- 🦀 **Rust** — безопасный и производительный код
- 🌐 **WebAssembly** — работа в браузере с нативной скоростью
- 📱 **Кроссплатформенность** — Web, Mobile Web, интеграция в Native

## Архитектура

```
mvp-qr-recognition/
├── crates/
│   ├── qr-core/          # Основная библиотека
│   │   ├── preprocessing # Предобработка изображений
│   │   ├── detection     # Обнаружение QR-кодов
│   │   ├── decoding      # Декодирование (rxing + rqrr)
│   │   └── payment       # Парсинг платёжных форматов
│   └── qr-wasm/          # WASM bindings
├── www/                  # Веб-демо
├── tests/                # Тестовые изображения
└── pkg/                  # Собранный WASM пакет
```

## Установка

### Требования
- [Rust](https://rustup.rs/) 1.70+
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- Node.js 18+

### Сборка

```bash
# Клонирование
git clone https://github.com/your-org/mvp-qr-recognition.git
cd mvp-qr-recognition

# Установка зависимостей
npm install

# Сборка WASM
npm run build

# Запуск демо
npm run dev
# Открыть http://localhost:3000
```

## Использование

### В браузере (ES Modules)

```javascript
import init, { WasmQRScanner } from './pkg/qr_wasm.js';

// Инициализация
await init();
const scanner = new WasmQRScanner();

// Сканирование файла
const response = await fetch('qr-image.png');
const bytes = new Uint8Array(await response.arrayBuffer());
const result = scanner.scanImage(bytes);

console.log(result);
// {
//   qr_codes: [{
//     content: "https://example.com",
//     content_type: "Url",
//     bbox: [100, 50, 200, 200],
//     confidence: 0.95
//   }],
//   best_payment: null,
//   processing_time_ms: 45
// }

// Сканирование из Canvas
const canvas = document.getElementById('canvas');
const ctx = canvas.getContext('2d');
const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);

const result2 = scanner.scanImageData(
  imageData.data,
  canvas.width,
  canvas.height
);

// Поиск платёжного QR
const payment = scanner.scanForPayment(bytes);
if (payment) {
  console.log('Платёж:', payment.amount, payment.currency);
}
```

### Настройки

```javascript
// Создание сканера с настройками
const scanner = WasmQRScanner.withConfig(
  true,   // adaptive_threshold
  51,     // block_size
  true,   // denoise
  1.0,    // denoise_strength
  true    // enhance_contrast
);
```

## Поддерживаемые платёжные форматы

### СБП (Система быстрых платежей)
```
https://qr.nspk.ru/AS1234?type=02&bank=100000000001&sum=10000&cur=RUB
```

### ST.00012 (Стандарт ЦБ РФ)
```
ST.00012|Name=ООО Тест|PersonalAcc=40817...|BIC=044525225|Sum=100000
```

### EMV QR Code
TLV-формат международных платёжных систем.

## API Reference

### WasmQRScanner

| Метод | Описание |
|-------|----------|
| `new()` | Создание сканера |
| `withConfig(...)` | Создание с настройками |
| `scanImage(bytes)` | Сканирование изображения |
| `scanImageData(data, w, h)` | Сканирование Canvas ImageData |
| `scanForPayment(bytes)` | Поиск платёжного QR |

### ScanResult

```typescript
interface ScanResult {
  qr_codes: QRResult[];
  best_payment: number | null;
  processing_time_ms: number;
}

interface QRResult {
  content: string;
  content_type: "Text" | "Url" | "Payment" | "VCard" | ...;
  bbox: [number, number, number, number];
  payment: PaymentInfo | null;
  confidence: number;
}

interface PaymentInfo {
  format: "EmvQR" | "SbpRussia" | "StRussia";
  payee_name?: string;
  amount?: number;
  currency?: string;
  bank?: string;
  purpose?: string;
}
```

## Разработка

```bash
# Запуск тестов
npm test

# WASM тесты в браузере
npm run test:wasm

# Линтер
npm run lint

# Очистка
npm run clean
```

## Лицензия

MIT
