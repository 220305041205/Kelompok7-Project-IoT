IoT Agriculture Monitoring System – Rust on ESP32-S3

Proyek ini merupakan pengembangan sistem pemantauan lingkungan pertanian berbasis IoT menggunakan mikrokontroler ESP32-S3. Seluruh sistem diprogram dengan bahasa Rust menggunakan pendekatan bare metal pada sistem operasi Ubuntu. Sensor yang digunakan adalah DHT11 untuk mengukur suhu dan kelembapan udara di sekitar lahan. Data hasil pembacaan sensor dikirim secara periodik ke cloud menggunakan protokol MQTT, dengan platform ThingsBoard sebagai sistem backend dan visualisasi datanya.

Lingkungan pengembangan dibangun di Ubuntu dengan konfigurasi toolchain Rust yang disesuaikan untuk arsitektur xtensa pada ESP32-S3. Tool yang digunakan antara lain espup, espflash, dan ldproxy sebagai jembatan antara Rust dan SDK ESP-IDF. Setelah konfigurasi toolchain selesai, firmware dikompilasi dan diflash langsung ke perangkat melalui port serial /dev/ttyACM0.

Program utama menginisialisasi sensor DHT11, mengatur koneksi Wi-Fi, serta membangun sesi komunikasi MQTT dengan ThingsBoard. Payload data berisi nilai suhu dan kelembapan dikirim secara kontinu melalui topic v1/devices/me/telemetry dengan format JSON. Akses ke server MQTT menggunakan access token dari ThingsBoard yang berfungsi sebagai autentikasi perangkat. Dashboard ThingsBoard menampilkan data real-time dalam bentuk grafik dan indikator status, sehingga memungkinkan pengguna memantau kondisi lingkungan pertanian dari jarak jauh.

Apabila koneksi Wi-Fi mengalami kegagalan atau terjadi error bertipe ESP_ERR_TIMEOUT, perlu dipastikan bahwa SSID dan password jaringan telah sesuai, serta sinyal Wi-Fi dapat dijangkau oleh modul ESP32-S3. Kegagalan deteksi perangkat pada proses flashing biasanya disebabkan oleh hak akses serial yang belum diatur, dan dapat diatasi dengan menambahkan pengguna ke grup dialout.

Secara keseluruhan, sistem ini merepresentasikan integrasi antara pemrograman bare metal berbasis Rust dengan ekosistem IoT modern melalui protokol MQTT dan layanan ThingsBoard Cloud. Implementasi ini menunjukkan bagaimana perangkat IoT berbasis embedded system dapat dikembangkan dengan efisien menggunakan bahasa pemrograman yang aman terhadap memory error, serta tetap mampu beroperasi pada sistem tertanam dengan sumber daya terbatas.


Langkah Eksekusi Singkat

1. Jalankan setup toolchain ESP32 untuk Rust:

   bash
   espup install
   . $HOME/export-esp.sh
   

2. Pindah ke direktori proyek dan lakukan build:

   bash
   cd ~/Documents/iot/projectiot
   cargo build --release --target xtensa-esp32s3-espidf
   

3. Flash firmware ke ESP32-S3 dan buka monitor real-time:

   bash
   espflash flash --monitor --port /dev/ttyACM0 target/xtensa-esp32s3-espidf/release/projectiot
   

4. Pastikan data sensor dan status koneksi MQTT muncul pada serial monitor, serta data streaming dapat diverifikasi melalui dashboard ThingsBoard.
