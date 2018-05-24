use std::collections::HashSet;

#[derive(Debug, Serialize, Deserialize)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    changed_pixels: HashSet<(u32, u32)>,
}

impl Canvas {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Canvas {
        Canvas {
            width,
            height,
            pixels,
            changed_pixels: HashSet::new(),
        }
    }

    pub fn blank(width: u32, height: u32) -> Canvas {
        let mut pixels = Vec::with_capacity((width * height * 3) as usize);
        pixels.resize((width * height * 3) as usize, 255);
        Self::new(width, height, pixels)
    }

    pub fn from_file(mut data: Vec<u8>) -> Option<Canvas> {
        if data.len() < 8 {
            return None;
        }
        let width: u32 = u32::from_be(unsafe { *(&data[0..4] as *const _ as *const u32) });
        let height: u32 = u32::from_be(unsafe { *(&data[4..8] as *const _ as *const u32) });
        let pixels = data.split_off(8);
        if (width * height) as usize * 3 != pixels.len() {
            return None;
        }
        Some(Self::new(width, height, pixels))
    }

    pub fn to_file(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(8);
        unsafe {
            vec.set_len(8);
            *(&mut vec[0..4] as *mut _ as *mut u32) = self.width.to_be();
            *(&mut vec[4..8] as *mut _ as *mut u32) = self.width.to_be();
        }
        vec.append(&mut self.pixels.clone());
        vec
    }

    fn index(&self, x: u32, y: u32) -> usize {
        (self.width * y + x) as usize * 3
    }

    pub fn region(&self, x: u32, y: u32, w: u32, h: u32) -> Option<Region> {
        if x >= self.width || x + w > self.width || y >= self.height || y + h > self.height {
            // nope
            return None;
        }

        let mut data = Vec::with_capacity((w * h) as usize * 3);

        for iy in y..(y + h) {
            data.extend_from_slice(&self.pixels[self.index(x, iy)..self.index(x + w, iy)]);
        }

        Some(Region { x, y, w, h, data })
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x >= self.width || y >= self.height {
            // nope
            return;
        }
        self.changed_pixels.insert((x, y));
        let index = self.index(x, y);
        self.pixels[index + 0] = r;
        self.pixels[index + 1] = g;
        self.pixels[index + 2] = b;
    }

    pub fn compile_deltas(&mut self, max_pixels: Option<usize>) -> Vec<Region> {
        let mut quad_tree = QuadNode::new(0, 0, self.width, self.height);

        let mut i = 0;
        for pixel in self.changed_pixels.clone() {
            if let Some(max_pixels) = max_pixels {
                if i == max_pixels {
                    break;
                }
            }

            quad_tree.insert_data(pixel.0, pixel.1);
            self.changed_pixels.remove(&pixel);

            i += 1;
        }

        quad_tree.reduce();
        quad_tree
            .regions()
            .iter()
            .map(|(x, y, width, height)| self.region(*x, *y, *width, *height).unwrap())
            .collect()
    }

    pub fn set_size(&mut self, new_width: u32, new_height: u32) {
        let mut new_pixels = Vec::with_capacity((new_width * new_height * 3) as usize);

        let max_x = if self.width > new_width {
            new_width
        } else {
            self.width
        };
        let add_x = new_width.saturating_sub(self.width);
        let mut add_x_pixels = Vec::new();
        add_x_pixels.resize((add_x * 3) as usize, 255);
        let mut full_x_pixels = Vec::new();
        full_x_pixels.resize((new_width * 3) as usize, 255);

        for y in 0..self.height {
            if y >= new_height {
                break;
            }

            new_pixels.extend_from_slice(&self.pixels[self.index(0, y)..self.index(max_x, y)]);
            new_pixels.extend_from_slice(&add_x_pixels);
        }

        for _ in self.height..new_height {
            new_pixels.extend_from_slice(&full_x_pixels);
        }

        self.width = new_width;
        self.height = new_height;
        self.pixels = new_pixels;
        self.changed_pixels.clear();
    }
}

/// QuadTree node.
///
/// ```
///   0 1
/// 0 a b
/// 1 c d
/// ```
#[derive(Debug)]
struct QuadNode {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    a: Option<Box<QuadNode>>,
    b: Option<Box<QuadNode>>,
    c: Option<Box<QuadNode>>,
    d: Option<Box<QuadNode>>,
    data: Vec<(u32, u32)>,
}

impl QuadNode {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> QuadNode {
        QuadNode {
            x,
            y,
            width,
            height,
            a: None,
            b: None,
            c: None,
            d: None,
            data: Vec::new(),
        }
    }

    pub fn mut_quad(&mut self, x: u32, y: u32) -> &mut Option<Box<QuadNode>> {
        let x_least = x < self.width / 2;
        let y_least = y < self.height / 2;
        match (x_least, y_least) {
            (true, true) => &mut self.a,
            (false, true) => &mut self.b,
            (true, false) => &mut self.c,
            (false, false) => &mut self.d,
        }
    }

    pub fn quad_rect(&self, x: u32, y: u32) -> (u32, u32, u32, u32) {
        let x_least = x < self.width / 2;
        let y_least = y < self.height / 2;
        (
            match x_least {
                true => self.x,
                false => self.x + self.width / 2,
            },
            match y_least {
                true => self.y,
                false => self.y + self.height / 2,
            },
            match x_least {
                true => self.width / 2,
                false => self.width - self.width / 2,
            },
            match y_least {
                true => self.height / 2,
                false => self.height - self.height / 2,
            },
        )
    }

    pub fn insert_data(&mut self, x: u32, y: u32) {
        if self.width == 1 && self.height == 1 {
            self.data = vec![(x, y)];
            return;
        }

        let quad_rect = self.quad_rect(x - self.x, y - self.y);
        let self_x = self.x;
        let self_y = self.y;
        let quad = self.mut_quad(x - self_x, y - self_y);
        if quad.is_none() {
            *quad = Some(Box::new(QuadNode::new(
                quad_rect.0,
                quad_rect.1,
                quad_rect.2,
                quad_rect.3,
            )));
        }

        quad.as_mut().unwrap().insert_data(x, y);
    }

    pub fn reduce(&mut self) -> usize {
        let mut data_count = self.data.len();

        if let Some(ref mut a) = self.a {
            data_count += a.reduce();
        }
        if let Some(ref mut b) = self.b {
            data_count += b.reduce();
        }
        if let Some(ref mut c) = self.c {
            data_count += c.reduce();
        }
        if let Some(ref mut d) = self.d {
            data_count += d.reduce();
        }

        let merge = data_count > (self.width * self.height / 8) as usize
            || (self.a.is_some() && self.b.is_some() && self.c.is_some() && self.d.is_some());

        if merge {
            if let Some(mut a) = self.a.take() {
                self.data.append(&mut a.data);
            }
            if let Some(mut b) = self.b.take() {
                self.data.append(&mut b.data);
            }
            if let Some(mut c) = self.c.take() {
                self.data.append(&mut c.data);
            }
            if let Some(mut d) = self.d.take() {
                self.data.append(&mut d.data);
            }
        }

        data_count
    }

    pub fn regions(&self) -> Vec<(u32, u32, u32, u32)> {
        let mut regions = Vec::new();

        if !self.data.is_empty() {
            regions.push((self.x, self.y, self.width, self.height));
        } else {
            if let Some(ref a) = self.a {
                regions.append(&mut a.regions());
            }
            if let Some(ref b) = self.b {
                regions.append(&mut b.regions());
            }
            if let Some(ref c) = self.c {
                regions.append(&mut c.regions());
            }
            if let Some(ref d) = self.d {
                regions.append(&mut d.regions());
            }
        }

        regions
    }
}
