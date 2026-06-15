<?php
/**
 * Generic fallback template (blog/news index, search results, archives).
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
get_header();
?>
<section class="page-banner">
	<div class="container">
		<h1><?php
			if ( is_home() ) { single_post_title( '', true ); }
			elseif ( is_search() ) { printf( esc_html__( 'Search: %s', 'allied' ), '<em>' . esc_html( get_search_query() ) . '</em>' ); }
			else { the_archive_title(); }
		?></h1>
	</div>
</section>

<section class="section">
	<div class="container">
		<?php if ( have_posts() ) : ?>
			<div class="grid grid--3">
				<?php while ( have_posts() ) : the_post(); ?>
					<article class="card">
						<a class="card__media" href="<?php the_permalink(); ?>" tabindex="-1" aria-hidden="true"><?php allied_thumbnail(); ?></a>
						<div class="card__body">
							<h3 class="card__title"><a href="<?php the_permalink(); ?>"><?php the_title(); ?></a></h3>
							<p class="text-muted"><?php echo esc_html( get_the_excerpt() ); ?></p>
						</div>
					</article>
				<?php endwhile; ?>
			</div>
			<div class="section--tight"><?php the_posts_pagination( array( 'mid_size' => 1 ) ); ?></div>
		<?php else : ?>
			<p class="lead"><?php esc_html_e( 'Nothing found.', 'allied' ); ?></p>
		<?php endif; ?>
	</div>
</section>
<?php get_footer(); ?>
