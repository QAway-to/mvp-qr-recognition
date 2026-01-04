/* tslint:disable */
/* eslint-disable */

export class WasmQRScanner {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Сканирование изображения из байтов (PNG, JPEG)
   * 
   * @param image_data - Uint8Array с данными изображения
   * @returns Object с результатами сканирования
   */
  scanImage(image_data: Uint8Array): any;
  /**
   * Создание сканера с пользовательскими настройками
   */
  static withConfig(adaptive_threshold: boolean, block_size: number, denoise: boolean, denoise_strength: number, enhance_contrast: boolean): WasmQRScanner;
  /**
   * Сканирование ImageData из Canvas
   * 
   * @param data - Uint8ClampedArray из canvas.getImageData()
   * @param width - Ширина изображения
   * @param height - Высота изображения
   * @returns Object с результатами сканирования
   */
  scanImageData(data: Uint8Array, width: number, height: number): any;
  /**
   * Поиск платёжного QR-кода
   * 
   * @param image_data - Uint8Array с данными изображения
   * @returns PaymentInfo или null
   */
  scanForPayment(image_data: Uint8Array): any;
  /**
   * Создание нового сканера с настройками по умолчанию
   */
  constructor();
}

/**
 * Удобная функция для быстрого сканирования
 */
export function quickScan(image_data: Uint8Array): any;

/**
 * Инициализация panic hook для отладки
 */
export function start(): void;

/**
 * Информация о версии
 */
export function version(): string;
