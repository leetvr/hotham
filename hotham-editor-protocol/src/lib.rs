use std::io::{Read, Write};

pub use openxr_sys::ViewConfigurationView;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestType {
    GetViewConfiguration,
    GetViewCount,
}

pub trait Request {
    type Response: Clone;
    fn request_type(&self) -> RequestType;
}

pub trait RequestWithVecResponse {
    type ResponseItem: Clone; // shitty name
}

pub mod requests {
    use crate::{responses::ViewConfiguration, Request, RequestType};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct GetViewConfiguration {}

    impl Request for GetViewConfiguration {
        type Response = ViewConfiguration;
        fn request_type(&self) -> RequestType {
            RequestType::GetViewConfiguration
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct GetViewCount {}

    impl Request for GetViewCount {
        type Response = u32;
        fn request_type(&self) -> RequestType {
            RequestType::GetViewCount
        }
    }
}

pub mod responses {
    #[derive(Debug, Clone, Copy)]
    pub struct ViewConfiguration {
        pub width: u32,
        pub height: u32,
    }
}

#[derive(Debug, Clone)]
pub struct RequestHeader {
    pub payload_length: u32,
    pub request_type: RequestType,
}

impl From<RequestHeader> for Vec<u8> {
    fn from(h: RequestHeader) -> Self {
        unsafe { bytes_from_t(&h).to_vec() }
    }
}

fn write_request<S: Write, R: Request>(request: &R, writer: &mut S) -> std::io::Result<()> {
    let header = RequestHeader {
        request_type: request.request_type(),
        payload_length: std::mem::size_of::<R>() as u32,
    };

    writer.write_all(&{ unsafe { bytes_from_t(&header) } })?;
    writer.write_all(&{ unsafe { bytes_from_t(request) } })?;

    Ok(())
}

fn read_request_header<'a, S: Read>(
    reader: &mut S,
    buf: &'a mut [u8],
) -> std::io::Result<RequestHeader> {
    reader.read_exact(&mut buf[..std::mem::size_of::<RequestHeader>()])?;
    let header: RequestHeader =
        unsafe { t_from_bytes(&mut buf[..std::mem::size_of::<RequestHeader>()]) };
    Ok(header)
}

fn read_request_payload<'a, R: Request + Clone, S: Read>(
    reader: &mut S,
    buf: &'a mut [u8],
    payload_length: usize,
) -> std::io::Result<R> {
    reader.read_exact(&mut buf[..payload_length])?;
    let payload = &buf[..payload_length];
    Ok(unsafe { t_from_bytes(payload) })
}

pub struct EditorClient<S> {
    socket: S,
    buffer: Vec<u8>,
}

impl<S: Read + Write> EditorClient<S> {
    pub fn new(socket: S) -> Self {
        Self {
            socket,
            buffer: vec![0; 1024 * 1024],
        }
    }

    pub fn request<R: Request>(&mut self, request: &R) -> std::io::Result<R::Response> {
        self.send_request(request)?;
        self.get_response::<R::Response>()
    }

    pub fn request_vec<R: Request + RequestWithVecResponse>(
        &mut self,
        request: &R,
    ) -> std::io::Result<Vec<R::ResponseItem>> {
        self.send_request(request)?;
        self.get_response_vec::<R::ResponseItem>()
    }

    pub fn send_request<R: Request>(&mut self, request: &R) -> std::io::Result<()> {
        write_request(request, &mut self.socket)
    }

    pub fn get_response<R: Clone>(&mut self) -> std::io::Result<R> {
        let socket = &mut self.socket;
        let buf = &mut self.buffer;

        let header_size = std::mem::size_of::<u32>();
        socket.read_exact(&mut buf[..header_size])?;
        let message_size = u32::from_be_bytes(buf[..header_size].try_into().unwrap()) as usize;

        self.socket.read_exact(&mut buf[..message_size])?;
        Ok(unsafe { t_from_bytes(&buf[..message_size]) })
    }

    pub fn get_response_vec<R: Clone>(&mut self) -> std::io::Result<Vec<R>> {
        let socket = &mut self.socket;
        let buf = &mut self.buffer;

        let header_size = std::mem::size_of::<u32>();
        socket.read_exact(&mut buf[..header_size])?;
        let message_size = u32::from_be_bytes(buf[..header_size].try_into().unwrap()) as usize;

        self.socket.read_exact(&mut buf[..message_size])?;
        Ok(unsafe { vec_from_bytes(&buf[..message_size]) })
    }
}

pub struct EditorServer<S> {
    socket: S,
    buffer: Vec<u8>,
}

impl<S: Read + Write> EditorServer<S> {
    pub fn new(socket: S) -> Self {
        Self {
            socket,
            buffer: vec![0; 1024 * 1024],
        }
    }

    /// Helpful if you already know in advance what the request type is
    pub fn get_request<R: Request + Clone>(&mut self) -> std::io::Result<R> {
        let request_header = read_request_header(&mut self.socket, &mut self.buffer)?;
        read_request_payload(
            &mut self.socket,
            &mut self.buffer,
            request_header.payload_length as usize,
        )
    }

    pub fn get_request_header(&mut self) -> std::io::Result<RequestHeader> {
        read_request_header(&mut self.socket, &mut self.buffer)
    }

    pub fn get_request_payload<R: Request + Clone>(
        &mut self,
        payload_length: u32,
    ) -> std::io::Result<R> {
        read_request_payload(&mut self.socket, &mut self.buffer, payload_length as usize)
    }

    pub fn send_response<T>(&mut self, response: &T) -> std::io::Result<()> {
        let message_size = std::mem::size_of::<T>() as u32;
        self.socket.write(&message_size.to_be_bytes())?;
        self.socket.write(&unsafe { bytes_from_t(response) })?;

        Ok(())
    }
}

unsafe fn vec_from_bytes<T: Clone>(data: &[u8]) -> Vec<T> {
    let len = data.len() / std::mem::size_of::<T>();
    std::slice::from_raw_parts(data.as_ptr().cast(), len).to_vec()
}

unsafe fn t_from_bytes<T: Clone>(data: &[u8]) -> T {
    std::ptr::read(data.as_ptr().cast::<T>()).clone()
}

unsafe fn bytes_from_t<T>(data: &T) -> Vec<u8> {
    std::slice::from_raw_parts(data as *const _ as *const u8, std::mem::size_of::<T>()).to_vec()
}

#[cfg(test)]
mod tests {
    use crate::requests::GetViewConfiguration;

    use super::*;
    use openxr_sys::{StructureType, ViewConfigurationView};
    use std::{cell::RefCell, rc::Rc};

    #[derive(Default, Clone)]
    struct MockSocket {
        pub data: Rc<RefCell<Vec<u8>>>,
    }

    impl MockSocket {
        pub fn reset(&self) {
            self.data.borrow_mut().clear();
        }
    }

    impl Write for MockSocket {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.data.borrow_mut().write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Read for MockSocket {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut data = self.data.borrow_mut();
            let read_length = buf.len();
            buf.copy_from_slice(&data[..buf.len()]);
            data.rotate_left(read_length); // shuffle the bytes that we just read back over to the end
            Ok(buf.len())
        }
    }

    #[test]
    pub fn test_request_response() {
        let socket = MockSocket::default();
        let mut client = EditorClient::new(socket.clone());
        let mut server = EditorServer::new(socket);

        let request = GetViewConfiguration {};
        client.send_request(&request).unwrap();
        let request_header = server.get_request_header().unwrap();
        match request_header.request_type {
            RequestType::GetViewConfiguration => {
                let request_from_client: GetViewConfiguration = server
                    .get_request_payload(request_header.payload_length)
                    .unwrap();
                assert_eq!(request, request_from_client)
            }
            _ => panic!("Bad request"),
        };

        server.socket.reset();

        let response = ViewConfigurationView {
            ty: StructureType::VIEW_CONFIGURATION_VIEW,
            next: std::ptr::null_mut(),
            recommended_image_rect_width: 100,
            max_image_rect_width: 100,
            recommended_image_rect_height: 100,
            max_image_rect_height: 100,
            recommended_swapchain_sample_count: 100,
            max_swapchain_sample_count: 100,
        };

        server.send_response(&response).unwrap();
        let response_from_server: ViewConfigurationView = client.get_response().unwrap();
        assert_eq!(response.ty, response_from_server.ty);
        assert_eq!(
            response.max_swapchain_sample_count,
            response_from_server.max_swapchain_sample_count
        );

        server.socket.reset();

        let response = [
            ViewConfigurationView {
                ty: StructureType::VIEW_CONFIGURATION_VIEW,
                next: std::ptr::null_mut(),
                recommended_image_rect_width: 100,
                max_image_rect_width: 100,
                recommended_image_rect_height: 100,
                max_image_rect_height: 100,
                recommended_swapchain_sample_count: 100,
                max_swapchain_sample_count: 100,
            },
            ViewConfigurationView {
                ty: StructureType::VIEW_CONFIGURATION_VIEW,
                next: std::ptr::null_mut(),
                recommended_image_rect_width: 200,
                max_image_rect_width: 100,
                recommended_image_rect_height: 100,
                max_image_rect_height: 100,
                recommended_swapchain_sample_count: 100,
                max_swapchain_sample_count: 100,
            },
        ];

        server.send_response(&response).unwrap();
        let response_from_server: Vec<ViewConfigurationView> = client.get_response_vec().unwrap();
        assert_eq!(response.len(), response_from_server.len());
        assert_eq!(
            response[0].max_swapchain_sample_count,
            response_from_server[0].max_swapchain_sample_count
        );
        assert_eq!(
            response[1].max_swapchain_sample_count,
            response_from_server[1].max_swapchain_sample_count
        );
    }
}
