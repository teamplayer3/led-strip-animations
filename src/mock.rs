pub struct SPI;

impl embedded_hal::spi::FullDuplex<u8> for SPI {
    type Error = ();

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        unimplemented!()
    }

    fn send(&mut self, _: u8) -> nb::Result<(), Self::Error> {
        unimplemented!()
    }
}
