if (!globalThis.TextDecoder) {
  globalThis.TextDecoder = class TextDecoder {
    decode(array) {
      if (typeof array !== 'undefined') {
        return String.fromCharCode(...array);
      } else {
        return '';
      }
    }
  };
}

if (!globalThis.TextEncoder) {
    globalThis.TextEncoder = class TextEncoder {
        encode(arg) {
            if (typeof arg !== 'undefined') {
                throw Error('TextEncoder stub called');
            } else {
                return new Uint8Array(0);
            }
        }
    };
}
